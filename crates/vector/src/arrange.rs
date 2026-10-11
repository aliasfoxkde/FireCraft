//! Align / distribute math (the Align panel's geometry): per-object
//! translations derived from the objects' bounding boxes. Pure math —
//! the engine resolves bounds, applies translations, and captures the
//! undo deltas; this module never touches documents.
//!
//! Align moves every object so its chosen box side coincides with that
//! side of the selection's union box. Distribute ("space evenly") keeps
//! the outermost objects fixed and slides the rest so the gaps between
//! adjacent boxes along the axis are equal — the Fireworks panel's
//! behavior, and self-consistent by construction (the uniform gap is
//! derived from the anchors' span).

use geom::{Rect, Vec2};

/// What to align each box to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignMode {
    /// Left edges onto the union's left edge.
    Left,
    /// Horizontal centers onto the union's center.
    CenterX,
    /// Right edges onto the union's right edge.
    Right,
    /// Top edges onto the union's top edge.
    Top,
    /// Vertical centers onto the union's center.
    CenterY,
    /// Bottom edges onto the union's bottom edge.
    Bottom,
}

/// Which axis `distribute` spaces along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributeAxis {
    /// Equal gaps between left/right edges.
    Horizontal,
    /// Equal gaps between top/bottom edges.
    Vertical,
}

/// Per-index translation aligning each box per `mode` against the
/// union of all boxes. An empty input yields an empty output; a single
/// box aligns to itself (zero translation). Input boxes are expected
/// finite — the engine validates that before calling.
#[must_use]
pub fn align(rects: &[Rect], mode: AlignMode) -> Vec<Vec2> {
    let Some(union) = geom::bounds::union_all(rects.iter().copied()) else {
        return Vec::new();
    };
    rects
        .iter()
        .map(|rect| match mode {
            AlignMode::Left => Vec2::new(union.x0 - rect.x0, 0.0),
            AlignMode::CenterX => {
                Vec2::new((union.x0 + union.x1) / 2.0 - (rect.x0 + rect.x1) / 2.0, 0.0)
            }
            AlignMode::Right => Vec2::new(union.x1 - rect.x1, 0.0),
            AlignMode::Top => Vec2::new(0.0, union.y0 - rect.y0),
            AlignMode::CenterY => {
                Vec2::new(0.0, (union.y0 + union.y1) / 2.0 - (rect.y0 + rect.y1) / 2.0)
            }
            AlignMode::Bottom => Vec2::new(0.0, union.y1 - rect.y1),
        })
        .collect()
}

/// Per-index translation spacing the boxes with equal gaps along
/// `axis`. The min-side and max-side boxes are the anchors and stay
/// put; with fewer than three boxes every box is an anchor, so all
/// translations are zero. Ties in position order break by far edge,
/// then by input index, so the result is deterministic.
#[must_use]
pub fn distribute(rects: &[Rect], axis: DistributeAxis) -> Vec<Vec2> {
    if rects.len() < 3 {
        return vec![Vec2::ZERO; rects.len()];
    }
    type Selector = fn(&Rect) -> f64;
    let (min_of, max_of, width_of): (Selector, Selector, Selector) = match axis {
        DistributeAxis::Horizontal => (|r| r.x0, |r| r.x1, |r| r.x1 - r.x0),
        DistributeAxis::Vertical => (|r| r.y0, |r| r.y1, |r| r.y1 - r.y0),
    };
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by(|&a, &b| {
        (min_of(&rects[a]), max_of(&rects[a]), a)
            .partial_cmp(&(min_of(&rects[b]), max_of(&rects[b]), b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let first = order[0];
    let last = order[order.len() - 1];
    // Equal gaps between adjacent edges across the anchors' span: the
    // gap absorbs exactly the slack between the boxes.
    let span = max_of(&rects[last]) - min_of(&rects[first]);
    let total: f64 = rects.iter().map(width_of).sum();
    let gap = (span - total) / (rects.len() - 1) as f64;

    let mut translations = vec![Vec2::ZERO; rects.len()];
    let mut cursor = max_of(&rects[first]) + gap;
    for &index in &order[1..order.len() - 1] {
        let rect = &rects[index];
        let shift = cursor - min_of(rect);
        translations[index] = match axis {
            DistributeAxis::Horizontal => Vec2::new(shift, 0.0),
            DistributeAxis::Vertical => Vec2::new(0.0, shift),
        };
        cursor = min_of(rect) + shift + width_of(rect) + gap;
    }
    translations
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect::new(x0, y0, x1, y1)
    }

    #[test]
    fn align_pulls_each_side_onto_the_union() {
        let rects = [rect(0.0, 0.0, 10.0, 10.0), rect(20.0, 5.0, 30.0, 15.0)];
        let got = align(&rects, AlignMode::Left);
        assert!((got[0].x).abs() < 1e-12);
        assert!((got[1].x - -20.0).abs() < 1e-12);
        assert!(got.iter().all(|t| t.y == 0.0));

        // CenterX: union spans 0..30, center 15; second box center 25.
        let got = align(&rects, AlignMode::CenterX);
        assert!((got[1].x - -10.0).abs() < 1e-12);

        // Right: union right edge 30; second box already there.
        let got = align(&rects, AlignMode::Right);
        assert!((got[0].x - 20.0).abs() < 1e-12);
        assert!(got[1].x.abs() < 1e-12);

        let stacked = [rect(0.0, 0.0, 10.0, 10.0), rect(2.0, 40.0, 8.0, 50.0)];
        let got = align(&stacked, AlignMode::CenterY);
        // Union 0..50 center 25; first box center 5 → +20.
        assert!((got[0].y - 20.0).abs() < 1e-12);
        assert!((got[1].y - -20.0).abs() < 1e-12);
    }

    #[test]
    fn distribute_spaces_the_middles_between_anchors() {
        // Boxes of width 4 at x = 0, 7, 30: span 34, total 12,
        // gaps = (34 - 12) / 2 = 11 → middles move to x0 = 15.
        let rects = [
            rect(0.0, 0.0, 4.0, 10.0),
            rect(7.0, 0.0, 11.0, 10.0),
            rect(30.0, 0.0, 34.0, 10.0),
        ];
        let got = distribute(&rects, DistributeAxis::Horizontal);
        assert_eq!(got[0], Vec2::ZERO);
        assert_eq!(got[2], Vec2::ZERO);
        assert!((got[1].x - 8.0).abs() < 1e-12, "middle to x0=15, was 7");

        // Unequal widths: gaps equal, positions differ.
        let rects = [
            rect(0.0, 0.0, 2.0, 0.0),
            rect(5.0, 0.0, 15.0, 0.0),
            rect(20.0, 0.0, 24.0, 0.0),
        ];
        let got = distribute(&rects, DistributeAxis::Horizontal);
        // Span 24, total 16, gap 4: middle x0 → 2 + 4 = 6, shift +1.
        assert!((got[1].x - 1.0).abs() < 1e-12);

        // Vertical axis mirrors.
        let rects = [
            rect(0.0, 0.0, 10.0, 4.0),
            rect(0.0, 7.0, 10.0, 11.0),
            rect(0.0, 30.0, 10.0, 34.0),
        ];
        let got = distribute(&rects, DistributeAxis::Vertical);
        assert_eq!(got[0], Vec2::ZERO);
        assert_eq!(got[1].x, 0.0);
        assert!((got[1].y - 8.0).abs() < 1e-12);
        assert_eq!(got[2], Vec2::ZERO);
    }

    #[test]
    fn distribute_is_order_independent_and_tie_broken() {
        let a = rect(0.0, 0.0, 4.0, 10.0);
        let b = rect(7.0, 0.0, 11.0, 10.0);
        let c = rect(30.0, 0.0, 34.0, 10.0);
        let forward = distribute(&[a, b, c], DistributeAxis::Horizontal);
        let shuffled = distribute(&[c, a, b], DistributeAxis::Horizontal);
        assert_eq!(forward[1].x, shuffled[2].x);
        // Identical boxes: tie-break by index keeps them in place.
        let same = distribute(&[a, a, a], DistributeAxis::Horizontal);
        assert!(same.iter().all(|t| *t == Vec2::ZERO));
    }

    #[test]
    fn degenerate_inputs_are_total() {
        assert!(align(&[], AlignMode::Top).is_empty());
        let one = [rect(0.0, 0.0, 5.0, 5.0)];
        assert!(align(&one, AlignMode::Top).iter().all(|t| *t == Vec2::ZERO));
        assert!(distribute(&[], DistributeAxis::Horizontal).is_empty());
        let two = [one[0], rect(10.0, 0.0, 15.0, 5.0)];
        assert!(
            distribute(&two, DistributeAxis::Horizontal)
                .iter()
                .all(|t| *t == Vec2::ZERO)
        );
    }
}
