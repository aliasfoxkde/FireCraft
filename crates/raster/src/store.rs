//! A width-by-height canvas of copy-on-write tiles.

use crate::tile::Tile;

/// Largest canvas side (pixels) a [`TileStore`] accepts.
pub const MAX_SIDE: u32 = 65_536;
/// Largest tile count a [`TileStore`] accepts (worst case 8 GiB of pixel
/// data). Caps exist because canvas dimensions arrive from untrusted files.
pub const MAX_TILES: u64 = 131_072;

/// Why a [`TileStore`] could not be created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreError {
    /// The value that exceeded the cap.
    pub got: u64,
    /// The cap that was exceeded.
    pub max: u64,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "raster store dimension {} exceeds cap {}",
            self.got, self.max
        )
    }
}

impl std::error::Error for StoreError {}

/// A `width x height` canvas of 128x128 RGBA8 tiles, row-major.
///
/// Right/bottom edge tiles are only partially covered by the canvas;
/// consumers clip against [`TileStore::width`] and [`TileStore::height`].
#[derive(Clone, Debug)]
pub struct TileStore {
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
    tiles: Vec<Tile>,
}

impl TileStore {
    /// Creates a zeroed store, or returns [`StoreError`] when the
    /// dimensions exceed the safety caps. Zero-sized canvases are valid
    /// (they simply own no tiles).
    ///
    /// # Errors
    /// [`StoreError`] when either side exceeds [`MAX_SIDE`] or the tile
    /// grid would exceed [`MAX_TILES`].
    pub fn new(width: u32, height: u32) -> Result<Self, StoreError> {
        let cols = u32::div_ceil(width, 128);
        let rows = u32::div_ceil(height, 128);
        let count = u64::from(cols) * u64::from(rows);
        if width > MAX_SIDE || height > MAX_SIDE {
            return Err(StoreError {
                got: u64::from(width.max(height)),
                max: u64::from(MAX_SIDE),
            });
        }
        if count > MAX_TILES {
            return Err(StoreError {
                got: count,
                max: MAX_TILES,
            });
        }
        let count = usize::try_from(count).map_err(|_| StoreError {
            got: count,
            max: MAX_TILES,
        })?;
        Ok(Self {
            width,
            height,
            cols,
            rows,
            tiles: vec![Tile::new(); count],
        })
    }

    /// Canvas width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Canvas height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Tile columns (ceil of width / 128).
    #[must_use]
    pub fn tile_cols(&self) -> u32 {
        self.cols
    }

    /// Tile rows (ceil of height / 128).
    #[must_use]
    pub fn tile_rows(&self) -> u32 {
        self.rows
    }

    /// Shared access to the tile at (`col`, `row`), or [`None`] when out of
    /// range.
    #[must_use]
    pub fn tile(&self, col: u32, row: u32) -> Option<&Tile> {
        let idx = self.index(col, row)?;
        self.tiles.get(idx)
    }

    /// Mutable access to the tile at (`col`, `row`); the store's clone
    ///-on-write applies per tile. [`None`] when out of range.
    pub fn tile_mut(&mut self, col: u32, row: u32) -> Option<&mut Tile> {
        let idx = self.index(col, row)?;
        self.tiles.get_mut(idx)
    }

    fn index(&self, col: u32, row: u32) -> Option<usize> {
        if col >= self.cols || row >= self.rows {
            return None;
        }
        usize::try_from(u64::from(row) * u64::from(self.cols) + u64::from(col)).ok()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn empty_canvas_owns_no_tiles() {
        let s = TileStore::new(0, 0).expect("zero canvas is valid");
        assert_eq!(s.tile_cols(), 0);
        assert_eq!(s.tile_rows(), 0);
        assert!(s.tile(0, 0).is_none());
    }

    #[test]
    fn exact_tile_multiple_has_no_clip_tiles() {
        let s = TileStore::new(256, 256).expect("2x2 tiles");
        assert_eq!(s.tile_cols(), 2);
        assert_eq!(s.tile_rows(), 2);
    }
}
