//! The shim-table layout: a row/column scan of the web objects'
//! bounds (the SLICES.XTT `slices[row][col]` grid) and the per-cell
//! assignment used by both the rasterizer and the HTML writer.

use crate::WebObject;

/// The scanned grid. `x_cuts`/`y_cuts` are pixel boundaries (length
/// `columns + 1` / `rows + 1`); `cells[row][col]` names the topmost
/// web object covering that cell's center, `None` when no object
/// does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    /// Column boundaries in pixels, ascending.
    pub x_cuts: Vec<u32>,
    /// Row boundaries in pixels, ascending.
    pub y_cuts: Vec<u32>,
    /// Per-cell topmost object index (into the web-object list).
    pub cells: Vec<Vec<Option<usize>>>,
}

impl Grid {
    /// Row count.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.y_cuts.len().saturating_sub(1)
    }

    /// Column count.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.x_cuts.len().saturating_sub(1)
    }

    /// The cell's pixel rect: `(x, y, width, height)`.
    #[must_use]
    pub fn cell_rect(&self, row: usize, col: usize) -> (u32, u32, u32, u32) {
        (
            self.x_cuts[col],
            self.y_cuts[row],
            self.x_cuts[col + 1] - self.x_cuts[col],
            self.y_cuts[row + 1] - self.y_cuts[row],
        )
    }
}

/// The point-containment test per `URSi` shape: rect frame, ellipse
/// inscribed in the frame, polygon over the contour anchors.
pub(crate) fn contains(slice: &doc::SliceObject, x: f64, y: f64) -> bool {
    let frame = &slice.frame;
    match slice.shape {
        1 => {
            let (cx, cy) = ((frame.x0 + frame.x1) / 2.0, (frame.y0 + frame.y1) / 2.0);
            let (rx, ry) = ((frame.x1 - frame.x0) / 2.0, (frame.y1 - frame.y0) / 2.0);
            if rx <= 0.0 || ry <= 0.0 {
                return false;
            }
            let dx = (x - cx) / rx;
            let dy = (y - cy) / ry;
            dx * dx + dy * dy <= 1.0
        }
        2 => {
            let points: Vec<(f64, f64)> = slice
                .contours
                .iter()
                .flat_map(|contour| {
                    contour
                        .points
                        .iter()
                        .map(|point| (point.anchor.x, point.anchor.y))
                })
                .collect();
            if points.len() < 3 {
                return false;
            }
            // Ray cast: crossing count parity.
            let mut inside = false;
            let mut j = points.len() - 1;
            for (i, &(xi, yi)) in points.iter().enumerate() {
                let (xj, yj) = points[j];
                if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                    inside = !inside;
                }
                j = i;
            }
            inside
        }
        _ => x >= frame.x0 && x <= frame.x1 && y >= frame.y0 && y <= frame.y1,
    }
}

/// One pixel edge of a frame, clamped into the canvas.
fn edge(value: f64, limit: u32) -> u32 {
    let clamped = value.round().clamp(0.0, f64::from(limit));
    // The clamp bounds are integers; the cast is exact.
    clamped as u32
}

/// Scans the web objects' bounds into the cut grid and assigns each
/// cell to its topmost object (later elements win). Objects render at
/// least one pixel; degenerate frames collapse to their clamped
/// edges.
pub(crate) fn scan(all_objects: &[WebObject<'_>], width: u16, height: u16) -> Grid {
    // Hotspots are map-only overlays: they take no cells and cut no
    // lines — their `<area>` rides whatever piece sits under them.
    // Cell values keep the full web-object index.
    let objects: Vec<(usize, &WebObject<'_>)> = all_objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.slice.slice_type.is_some())
        .collect();
    let mut xs: Vec<u32> = Vec::new();
    let mut ys: Vec<u32> = Vec::new();
    for (_, object) in &objects {
        let frame = &object.slice.frame;
        let (x0, y0) = (
            edge(frame.x0, u32::from(width)),
            edge(frame.y0, u32::from(height)),
        );
        let (x1, y1) = (
            edge(frame.x1, u32::from(width)),
            edge(frame.y1, u32::from(height)),
        );
        let (x0, x1) = (x0.min(x1), x0.max(x1).max(x0.min(x1) + 1));
        let (y0, y1) = (y0.min(y1), y0.max(y1).max(y0.min(y1) + 1));
        xs.extend([x0, x1]);
        ys.extend([y0, y1]);
    }
    xs.push(0);
    xs.push(u32::from(width));
    ys.push(0);
    ys.push(u32::from(height));
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

    let mut cells = vec![vec![None; xs.len().saturating_sub(1)]; ys.len().saturating_sub(1)];
    for (index, object) in &objects {
        let object = *object;
        let frame = &object.slice.frame;
        // The same clamp-and-stretch the cut scan used, so a
        // degenerate frame owns exactly its one-pixel band.
        let (x0, y0) = (
            f64::from(edge(frame.x0, u32::from(width))),
            f64::from(edge(frame.y0, u32::from(height))),
        );
        let (x1, y1) = (
            f64::from(edge(frame.x1, u32::from(width))),
            f64::from(edge(frame.y1, u32::from(height))),
        );
        let (x0, x1) = (x0.min(x1), x0.max(x1).max(x0.min(x1) + 1.0));
        let (y0, y1) = (y0.min(y1), y0.max(y1).max(y0.min(y1) + 1.0));
        for (row, window) in cells.iter_mut().enumerate() {
            let (cy0, cy1) = (f64::from(ys[row]), f64::from(ys[row + 1]));
            if cy1 <= y0 || cy0 >= y1 {
                continue;
            }
            let cy = (cy0 + cy1) / 2.0;
            for (col, cell) in window.iter_mut().enumerate() {
                let (cx0, cx1) = (f64::from(xs[col]), f64::from(xs[col + 1]));
                if cx1 <= x0 || cx0 >= x1 {
                    continue;
                }
                let cx = (cx0 + cx1) / 2.0;
                // Rects test against the clamped, one-pixel-stretched
                // band (degenerate frames still own a pixel);
                // ellipse/polygon use the raw-shape containment.
                let inside = match object.slice.shape {
                    1 | 2 => contains(object.slice, cx, cy),
                    _ => cx >= x0 && cx < x1 && cy >= y0 && cy < y1,
                };
                if inside {
                    *cell = Some(*index);
                }
            }
        }
    }
    Grid {
        x_cuts: xs,
        y_cuts: ys,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WebObject;
    use doc::{Contour, PathPoint, SliceObject};

    /// A rect slice over the frame.
    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> SliceObject {
        SliceObject {
            frame: geom::Rect::new(x0, y0, x1, y1),
            slice_type: Some(0),
            ..SliceObject::default()
        }
    }

    fn objects(slices: &[SliceObject]) -> Vec<WebObject<'_>> {
        slices.iter().map(|slice| WebObject { slice }).collect()
    }

    #[test]
    fn contains_matches_the_ursi_shape() {
        let square = rect(0.0, 0.0, 10.0, 10.0);
        assert!(contains(&square, 5.0, 5.0));
        assert!(!contains(&square, 11.0, 5.0));

        let ellipse = SliceObject {
            shape: 1,
            ..rect(0.0, 0.0, 10.0, 10.0)
        };
        assert!(contains(&ellipse, 5.0, 5.0));
        // The frame corner is outside the inscribed ellipse.
        assert!(!contains(&ellipse, 9.5, 9.5));

        let triangle = SliceObject {
            shape: 2,
            frame: geom::Rect::new(0.0, 0.0, 10.0, 10.0),
            contours: vec![Contour {
                closed: true,
                point_count: 3,
                points: vec![
                    PathPoint {
                        anchor: geom::Point::new(0.0, 0.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                    PathPoint {
                        anchor: geom::Point::new(10.0, 0.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                    PathPoint {
                        anchor: geom::Point::new(0.0, 10.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                ],
            }],
            ..SliceObject::default()
        };
        assert!(contains(&triangle, 1.0, 1.0));
        assert!(!contains(&triangle, 9.0, 9.0));
    }

    #[test]
    fn scan_cuts_on_bounds_and_assigns_topmost() {
        // Two stacked slices; the later one paints on top where they
        // overlap.
        let slices = [rect(0.0, 0.0, 4.0, 8.0), rect(2.0, 0.0, 6.0, 4.0)];
        let grid = scan(&objects(&slices), 8, 8);
        assert_eq!(grid.x_cuts, vec![0, 2, 4, 6, 8]);
        assert_eq!(grid.y_cuts, vec![0, 4, 8]);
        // Column bands: 0-2 only slice 0; 2-4 both (topmost 1);
        // 4-6 only slice 1; 6-8 neither.
        assert_eq!(grid.cells[0][0], Some(0));
        assert_eq!(grid.cells[0][1], Some(1));
        assert_eq!(grid.cells[0][2], Some(1));
        assert_eq!(grid.cells[0][3], None);
    }

    #[test]
    fn scan_never_produces_empty_cells() {
        // A degenerate (zero-width) frame still renders one pixel.
        let slices = [rect(3.0, 3.0, 3.0, 3.0)];
        let grid = scan(&objects(&slices), 8, 8);
        assert!(grid.cells.iter().flatten().any(|cell| cell.is_some()));
    }
}
