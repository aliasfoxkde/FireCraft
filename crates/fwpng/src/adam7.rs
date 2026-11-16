//! Adam7 interlacing: the seven passes an interlaced PNG carries.
//!
//! An interlaced PNG stores the same pixels as a non-interlaced one,
//! but split into seven passes that a viewer can decode and show
//! progressively: a coarse pass first (every eighth pixel), then
//! passes that fill in the gaps. The `png` crate can *read* such a
//! stream but cannot write one, so this module builds the pass stream
//! itself — the pass geometry, and the per-row filters PNG applies
//! to the pixels of each pass.
//!
//! The geometry is fixed by the PNG specification: pass *i* takes
//! the pixels whose x coordinate is `x_start + k * x_step` and whose
//! y coordinate is `y_start + m * y_step`, for the seven
//! `(x_start, y_start, x_step, y_step)` tuples below.

/// The seven Adam7 passes as `(x_start, y_start, x_step, y_step)`.
pub const PASSES: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

/// Bytes per pixel in the RGBA8 layout this module filters.
const BYTES_PER_PIXEL: usize = 4;

/// The pixels a pass holds along one axis: `ceil((extent - start) / step)`.
///
/// A pass that starts beyond the image holds nothing and is omitted
/// from the stream entirely.
fn pass_extent(extent: u32, start: u32, step: u32) -> u32 {
    if extent <= start {
        0
    } else {
        (extent - start).div_ceil(step)
    }
}

/// The filtered scanline stream of an interlaced RGBA8 image.
///
/// The stream is the concatenation of the seven passes; each pass
/// contributes its own rows, each row a filter byte followed by the
/// filtered row bytes. A pass's first row filters against zeros, as
/// the specification requires.
#[must_use]
pub fn scanlines(width: u32, height: u32, rgba8: &[u8]) -> Vec<u8> {
    let mut stream = Vec::new();
    for &(x_start, y_start, x_step, y_step) in &PASSES {
        let pass_width = pass_extent(width, x_start, x_step);
        let pass_height = pass_extent(height, y_start, y_step);
        if pass_width == 0 || pass_height == 0 {
            continue;
        }
        let mut prior = vec![0u8; pass_width as usize * BYTES_PER_PIXEL];
        for row in 0..pass_height {
            let y = y_start + row * y_step;
            let current = pass_row(width, rgba8, y, x_start, x_step, pass_width);
            let (filter, filtered) = filter_row(&current, &prior);
            stream.push(filter);
            stream.extend_from_slice(&filtered);
            prior = current;
        }
    }
    stream
}

/// One row of a pass, gathered from the image.
fn pass_row(
    width: u32,
    rgba8: &[u8],
    y: u32,
    x_start: u32,
    x_step: u32,
    pass_width: u32,
) -> Vec<u8> {
    let mut row = Vec::with_capacity(pass_width as usize * BYTES_PER_PIXEL);
    for x in 0..pass_width {
        let source_x = x_start + x * x_step;
        let offset = (y as usize * width as usize + source_x as usize) * BYTES_PER_PIXEL;
        row.extend_from_slice(&rgba8[offset..offset + BYTES_PER_PIXEL]);
    }
    row
}

/// Filters one row, choosing the filter PNG's heuristic prefers.
///
/// The heuristic is the specification's: for each of the five filter
/// types, score the row by the sum of the absolute values of the
/// filtered bytes read as signed, and take the smallest score. Ties
/// keep the earlier filter, so the choice is deterministic.
fn filter_row(current: &[u8], prior: &[u8]) -> (u8, Vec<u8>) {
    let mut best = (0u8, apply_filter(0, current, prior));
    let mut best_score = score(&best.1);
    for kind in 1u8..=4 {
        let filtered = apply_filter(kind, current, prior);
        let candidate = score(&filtered);
        if candidate < best_score {
            best_score = candidate;
            best = (kind, filtered);
        }
    }
    best
}

/// Applies one PNG row filter.
///
/// `0` is None, `1` Sub, `2` Up, `3` Average, `4` Paeth. Bytes
/// beyond the first pixel of a row use a zero left/up-left byte.
fn apply_filter(kind: u8, current: &[u8], prior: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; current.len()];
    for (index, &byte) in current.iter().enumerate() {
        let left = if index >= BYTES_PER_PIXEL {
            current[index - BYTES_PER_PIXEL]
        } else {
            0
        };
        let up = prior[index];
        let up_left = if index >= BYTES_PER_PIXEL {
            prior[index - BYTES_PER_PIXEL]
        } else {
            0
        };
        let predicted = match kind {
            1 => left,
            2 => up,
            3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
            4 => paeth(left, up, up_left),
            _ => 0,
        };
        out[index] = byte.wrapping_sub(predicted);
    }
    out
}

/// The PNG Paeth predictor.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(up_left);
    let to_left = (estimate - i16::from(left)).abs();
    let to_up = (estimate - i16::from(up)).abs();
    let to_up_left = (estimate - i16::from(up_left)).abs();
    if to_left <= to_up && to_left <= to_up_left {
        left
    } else if to_up <= to_up_left {
        up
    } else {
        up_left
    }
}

/// The heuristic's score for a filtered row: the sum of the absolute
/// values of its bytes, read as signed.
fn score(filtered: &[u8]) -> u64 {
    filtered
        .iter()
        .map(|byte| u64::from((*byte as i8).unsigned_abs()))
        .sum()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// A gradient image whose pixels differ on every coordinate, so
    /// every pass carries distinguishable content.
    fn gradient(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                out.push((x * 7) as u8);
                out.push((y * 11) as u8);
                out.push(((x + y) * 3) as u8);
                out.push(255);
            }
        }
        out
    }

    #[test]
    fn the_pass_geometry_matches_the_specification() {
        assert_eq!(pass_extent(8, 0, 8), 1);
        assert_eq!(pass_extent(9, 0, 8), 2);
        assert_eq!(pass_extent(8, 4, 8), 1);
        // The pass holds every pixel at `start + k * step` the image
        // covers, so a step that overshoots the image does not empty
        // a pass whose start is still inside it: pixel 4 of a
        // seven-pixel-wide image is in the pass.
        assert_eq!(pass_extent(7, 4, 8), 1, "the start is inside");
        assert_eq!(pass_extent(4, 4, 8), 0, "the edge is empty");
        assert_eq!(pass_extent(3, 4, 8), 0, "beyond the image is empty");
        assert_eq!(pass_extent(16, 0, 8), 2);
        assert_eq!(pass_extent(16, 0, 4), 4);
    }

    #[test]
    fn every_pass_contributes_its_rows() {
        let width = 9u32;
        let height = 9u32;
        let pixels = gradient(width, height);
        let stream = scanlines(width, height, &pixels);
        // Every pass of a 9x9 image holds at least one pixel, so the
        // stream carries seven passes' rows.
        let expected_rows: usize = PASSES
            .iter()
            .map(|&(x, y, xs, ys)| {
                let pw = pass_extent(width, x, xs) as usize;
                let ph = pass_extent(height, y, ys) as usize;
                ph * (1 + pw * BYTES_PER_PIXEL)
            })
            .sum();
        assert_eq!(stream.len(), expected_rows);
    }

    #[test]
    fn a_one_pixel_image_carries_one_pass() {
        let pixels = gradient(1, 1);
        let stream = scanlines(1, 1, &pixels);
        // Pass 1 holds the single pixel; the other six are empty.
        assert_eq!(stream.len(), 1 + BYTES_PER_PIXEL);
        assert_eq!(stream[0], 0, "a one-row pass filters against zeros");
    }

    #[test]
    fn the_filter_undoes_to_the_original_bytes() {
        let current: Vec<u8> = (0..32u8).collect();
        let prior: Vec<u8> = (100..132u8).collect();
        for kind in 0u8..=4 {
            let filtered = apply_filter(kind, &current, &prior);
            for (index, &byte) in current.iter().enumerate() {
                let left = if index >= BYTES_PER_PIXEL {
                    current[index - BYTES_PER_PIXEL]
                } else {
                    0
                };
                let up = prior[index];
                let up_left = if index >= BYTES_PER_PIXEL {
                    prior[index - BYTES_PER_PIXEL]
                } else {
                    0
                };
                let predicted = match kind {
                    1 => left,
                    2 => up,
                    3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                    4 => paeth(left, up, up_left),
                    _ => 0,
                };
                assert_eq!(filtered[index], byte.wrapping_sub(predicted));
            }
        }
    }
}
