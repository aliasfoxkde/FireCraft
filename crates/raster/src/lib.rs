//! Tiled raster storage for FireCraft.
//!
//! Layer L0: depends on nothing from this workspace. Tiles are 128x128 RGBA8
//! blocks — the same geometry as the Fireworks `mkBT` tile cache — held in
//! copy-on-write `Arc` storage so snapshots of a document are cheap and
//! edits deep-copy only the tile actually touched.

pub mod store;
pub mod tile;

pub use store::{StoreError, TileStore};
pub use tile::{TILE_BYTES, TILE_PIXELS, TILE_SIDE, Tile};

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{TILE_BYTES, Tile, TileStore};

    #[test]
    fn tile_cow_semantics() {
        let mut a = Tile::new();
        let b = a.clone();
        assert_eq!(a.shared_count(), 2, "clone must share storage");

        a.pixels_mut()[0] = 0xAB;
        a.pixels_mut()[1] = 0xCD;

        assert_eq!(a.shared_count(), 1, "write must detach the shared buffer");
        assert_eq!(
            b.shared_count(),
            1,
            "unwritten clone must keep the original buffer"
        );
        assert_eq!(b.pixels()[0], 0, "unwritten clone must not observe writes");
        assert_eq!(a.pixels()[0], 0xAB);
        assert_eq!(a.pixels()[1], 0xCD);
    }

    #[test]
    fn store_tiles_are_addressable_and_cow() {
        let mut store = TileStore::new(300, 200).expect("within caps");
        assert_eq!(store.tile_cols(), 3);
        assert_eq!(store.tile_rows(), 2);

        store
            .tile_mut(2, 1)
            .expect("tile 2,1 exists for a 300x200 store")
            .pixels_mut()[0] = 7;

        let snapshot = store.clone();
        store
            .tile_mut(2, 1)
            .expect("tile still present")
            .pixels_mut()[0] = 9;

        assert_eq!(
            snapshot.tile(2, 1).expect("snapshot tile").pixels()[0],
            7,
            "snapshot must not observe later writes"
        );
        assert_eq!(store.tile(2, 1).expect("live tile").pixels()[0], 9);
        assert_eq!(snapshot.tile(0, 0).expect("untouched tile").pixels()[0], 0);
        assert_eq!(
            snapshot.tile(0, 0).expect("tile").pixels().len(),
            TILE_BYTES
        );
    }

    #[test]
    fn store_rejects_oversized_dimensions() {
        assert!(TileStore::new(65_537, 1).is_err());
        assert!(TileStore::new(1, 65_537).is_err());
        // 65536/128 = 512 tiles per side; 512*512 = 262144 tiles is over the
        // tile-count cap and must be refused before any allocation.
        assert!(TileStore::new(65_536, 65_536).is_err());
    }

    #[test]
    fn out_of_range_tile_access_is_none() {
        let mut store = TileStore::new(100, 100).expect("small store");
        assert!(store.tile(1, 0).is_none());
        assert!(store.tile(0, 1).is_none());
        assert!(store.tile_mut(1, 0).is_none());
    }
}
