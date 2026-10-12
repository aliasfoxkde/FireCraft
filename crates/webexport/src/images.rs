//! Region rasters: one image file per table piece, through the M8
//! codecs, honoring each slice's `EXPv` override (`FMTi`/`JPQi`/
//! `FCLi`). Slices without an override export PNG — the exported
//! pixels are then exactly the composer's.

use crate::layout::{self, Grid};
use crate::naming;
use crate::{ExportedFile, WebExportError, WebExportOptions, WebObject};

/// One emitted table piece: the merged rectangle of same-object
/// cells, the object it belongs to (`None` = an area without slices)
/// and the file its `<img>` references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    /// Top-left cell of the piece.
    pub row: usize,
    /// Top-left cell of the piece.
    pub col: usize,
    /// Merged cell height.
    pub row_span: usize,
    /// Merged cell width.
    pub col_span: usize,
    /// Web-object index, `None` for an undefined-area piece.
    pub object: Option<usize>,
    /// Image file name the piece references.
    pub file: String,
}

/// The full canvas as straight-alpha RGBA8 (crop source).
pub(crate) fn straight_rgba(rendered: &compose::Render) -> Vec<u8> {
    compose::straight_alpha(&rendered.pixels)
}

/// Greedy maximal-rectangle decomposition of the assigned grid,
/// row-major. Equal-object runs extend right then down; skipped
/// (`include_undefined == false`) undefined cells produce nothing.
#[must_use]
pub fn decompose(
    grid: &Grid,
    include_undefined: bool,
) -> Vec<(usize, usize, usize, usize, Option<usize>)> {
    let (rows, cols) = (grid.rows(), grid.columns());
    let mut consumed = vec![vec![false; cols]; rows];
    let mut out = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            if consumed[row][col] {
                continue;
            }
            let object = grid.cells[row][col];
            if object.is_none() && !include_undefined {
                continue;
            }
            let mut col_end = col;
            while col_end + 1 < cols
                && !consumed[row][col_end + 1]
                && grid.cells[row][col_end + 1] == object
            {
                col_end += 1;
            }
            let mut row_end = row;
            'down: while row_end + 1 < rows {
                let next = &grid.cells[row_end + 1];
                let next_consumed = &consumed[row_end + 1];
                if (col..=col_end).any(|cc| next_consumed[cc] || next[cc] != object) {
                    break 'down;
                }
                row_end += 1;
            }
            for marks in &mut consumed[row..=row_end] {
                for mark in &mut marks[col..=col_end] {
                    *mark = true;
                }
            }
            out.push((row, col, row_end - row + 1, col_end - col + 1, object));
        }
    }
    out
}

/// The piece's codec from the slice's `EXPv` override: `0` gif, `1`
/// jpeg, anything else png (the no-override default included).
fn extension_of(slice: Option<&doc::SliceObject>) -> &'static str {
    match slice.and_then(|slice| slice.export.as_ref().and_then(|export| export.format)) {
        Some(0) => "gif",
        Some(1) => "jpg",
        _ => "png",
    }
}

/// Crops one region from the straight-alpha canvas, applying the
/// shape mask (pixel-center containment) for ellipse/polygon slices.
pub(crate) fn crop(
    canvas: &[u8],
    canvas_w: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    mask: Option<&doc::SliceObject>,
) -> Vec<u8> {
    let mut out = vec![0u8; w as usize * h as usize * 4];
    for row in 0..h {
        for col in 0..w {
            let (px, py) = (x + col, y + row);
            let src = (py as usize * canvas_w as usize + px as usize) * 4;
            let dst = (row as usize * w as usize + col as usize) * 4;
            if src + 4 > canvas.len() {
                continue;
            }
            let keep = match mask {
                None => true,
                Some(slice) => layout::contains(slice, f64::from(px) + 0.5, f64::from(py) + 0.5),
            };
            if keep {
                out[dst..dst + 4].copy_from_slice(&canvas[src..src + 4]);
            }
        }
    }
    out
}

/// Encodes one region per its format: GIF (single frame), JPEG
/// (flattened over the matte), PNG.
fn encode(
    rgba: Vec<u8>,
    w: u32,
    h: u32,
    slice: Option<&doc::SliceObject>,
) -> Result<Vec<u8>, WebExportError> {
    let plane =
        io::Plane::new(w, h, &rgba).map_err(|error| WebExportError::Encode(error.to_string()))?;
    let export = slice.and_then(|slice| slice.export.as_ref());
    match export.and_then(|export| export.format) {
        Some(0) => {
            io::gif::encode_frame(&plane).map_err(|error| WebExportError::Encode(error.to_string()))
        }
        Some(1) => {
            let mut options = io::jpeg::JpegOptions::default();
            if let Some(quality) = export.and_then(|export| export.jpeg_quality) {
                options.quality = quality.clamp(1, 100) as u8;
            }
            if let Some(matte) = export.and_then(|export| export.matte_raw) {
                let matte = matte as u32;
                options.matte = io::Matte::Rgb(
                    (matte & 0xFF) as u8,
                    ((matte >> 8) & 0xFF) as u8,
                    ((matte >> 16) & 0xFF) as u8,
                );
            }
            io::jpeg::encode(&plane, &options)
                .map_err(|error| WebExportError::Encode(error.to_string()))
        }
        _ => io::png::encode(&plane).map_err(|error| WebExportError::Encode(error.to_string())),
    }
}

/// The 1×1 transparent GIF the shim cells reference.
pub(crate) fn shim() -> Result<Vec<u8>, WebExportError> {
    let plane = io::Plane::new(1, 1, &[0, 0, 0, 0])
        .map_err(|error| WebExportError::Encode(error.to_string()))?;
    io::gif::encode_frame(&plane).map_err(|error| WebExportError::Encode(error.to_string()))
}

/// Rasterizes every piece of the grid into image files, in piece
/// order. With `emit_images` false the run only plans the pieces
/// (the CSS-sprites path encodes its own sheet instead).
///
/// # Errors
/// [`WebExportError::Encode`] when a codec refuses a region.
pub(crate) fn rasterize(
    options: &WebExportOptions,
    objects: &[WebObject<'_>],
    grid: &Grid,
    canvas: &[u8],
    canvas_w: u16,
    emit_images: bool,
) -> Result<(Vec<ExportedFile>, Vec<Piece>), WebExportError> {
    let multi = grid.rows() > 1 || grid.columns() > 1;
    let mut files = Vec::new();
    let mut pieces = Vec::new();
    let mut used_names = std::collections::BTreeSet::new();
    let mut per_object: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();
    for (row, col, row_span, col_span, object) in decompose(grid, options.include_undefined) {
        let (x, y, w, h) = grid.cell_rect(row, col);
        if w == 0 || h == 0 {
            continue;
        }
        let slice = object.map(|index| objects[index].slice);
        // Naming: a named object keeps its cleaned name across its
        // pieces (extra pieces get `_2`, `_3`, ... — possible only
        // where slices overlap); everything else auto-names from the
        // piece's top-left cell.
        let named = object
            .zip(slice)
            .and_then(|(index, slice)| slice.name.as_deref().map(|raw| (index, naming::clean(raw))))
            .filter(|(_, base)| !base.is_empty());
        let mut stem = match named {
            Some((index, base)) => {
                let counter = per_object.entry(index).or_insert(0);
                *counter += 1;
                if *counter == 1 {
                    base
                } else {
                    format!("{base}_{counter}")
                }
            }
            None => naming::auto_name(&options.doc_name, row, col, multi),
        };
        let extension = extension_of(slice);
        loop {
            if used_names.insert(naming::with_extension(&stem, extension)) {
                break;
            }
            stem = format!("{stem}_x");
        }
        let file = naming::with_extension(&stem, extension);
        if emit_images {
            let rgba = crop(
                canvas,
                u32::from(canvas_w),
                x,
                y,
                w,
                h,
                slice.filter(|slice| slice.shape != 0),
            );
            files.push(ExportedFile {
                name: file.clone(),
                bytes: encode(rgba, w, h, slice)?,
            });
        }
        pieces.push(Piece {
            row,
            col,
            row_span,
            col_span,
            object,
            file,
        });
    }
    Ok((files, pieces))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::WebObject;
    use doc::SliceObject;

    fn grid_of(cells: Vec<Vec<Option<usize>>>) -> Grid {
        let cols = cells.first().map_or(0, Vec::len);
        let rows = cells.len();
        Grid {
            x_cuts: (0..=cols as u32).map(|c| c * 10).collect(),
            y_cuts: (0..=rows as u32).map(|r| r * 10).collect(),
            cells,
        }
    }

    #[test]
    fn decompose_merges_same_object_runs() {
        // A 2x2 block of object 0 and two separate object-1 cells.
        let grid = grid_of(vec![
            [Some(0), Some(0), Some(1)].to_vec(),
            [Some(0), Some(0), None].to_vec(),
            [Some(1), None, None].to_vec(),
        ]);
        assert_eq!(
            decompose(&grid, true),
            vec![
                (0, 0, 2, 2, Some(0)),
                (0, 2, 1, 1, Some(1)),
                (1, 2, 2, 1, None),
                (2, 0, 1, 1, Some(1)),
                (2, 1, 1, 1, None),
            ]
        );
    }

    #[test]
    fn decompose_skips_undefined_when_asked() {
        let grid = grid_of(vec![[Some(0), None].to_vec(), [None, None].to_vec()]);
        assert_eq!(decompose(&grid, false), vec![(0, 0, 1, 1, Some(0))]);
        // Undefined cells still decompose as their own pieces.
        assert_eq!(decompose(&grid, true).len(), 3);
    }

    #[test]
    fn extensions_follow_the_expv_format() {
        let plain = SliceObject::default();
        assert_eq!(extension_of(Some(&plain)), "png");
        let gif = SliceObject {
            export: Some(doc::ExportOptions {
                format: Some(0),
                ..doc::ExportOptions::default()
            }),
            ..SliceObject::default()
        };
        assert_eq!(extension_of(Some(&gif)), "gif");
        let jpg = SliceObject {
            export: Some(doc::ExportOptions {
                format: Some(1),
                ..doc::ExportOptions::default()
            }),
            ..SliceObject::default()
        };
        assert_eq!(extension_of(Some(&jpg)), "jpg");
    }

    #[test]
    fn rasterize_names_and_dedups_files() {
        let slices = [
            SliceObject {
                name: Some("logo".to_owned()),
                ..SliceObject::default()
            },
            SliceObject::default(),
        ];
        let objects: Vec<WebObject<'_>> = slices.iter().map(|slice| WebObject { slice }).collect();
        let grid = grid_of(vec![[Some(0), Some(0)].to_vec(), [Some(1), None].to_vec()]);
        // 2x2 opaque checker so every crop encodes.
        let canvas_w = 20u32;
        let mut canvas = Vec::new();
        for row in 0..20u32 {
            for col in 0..canvas_w {
                let on = (col / 5 + row / 5) % 2 == 0;
                let value = if on { 255u8 } else { 0 };
                canvas.extend([value, 0, 0, 255]);
            }
        }
        let options = WebExportOptions::default();
        let (files, pieces) = rasterize(&options, &objects, &grid, &canvas, 20, true).unwrap();
        assert_eq!(pieces.len(), 3);
        assert_eq!(files[0].name, "logo.png");
        // Auto names carry the grid suffix.
        assert_eq!(files[1].name, "ndoc_2_1.png");
        // Undefined cells take their own image.
        assert_eq!(files[2].name, "ndoc_2_2.png");
    }
}
