//! The tile primitive: 128x128 RGBA8, copy-on-write.

use std::sync::Arc;

/// Tile edge length in pixels — the Fireworks `mkBT` tile geometry.
pub const TILE_SIDE: usize = 128;
/// Pixels per tile.
pub const TILE_PIXELS: usize = TILE_SIDE * TILE_SIDE;
/// Bytes per tile (RGBA8).
pub const TILE_BYTES: usize = TILE_PIXELS * 4;

/// A 128x128 RGBA8 tile with copy-on-write semantics.
///
/// Cloning a [`Tile`] is an atomic pointer bump; the first
/// [`Tile::pixels_mut`] call on a shared tile deep-copies the 64 KiB buffer.
/// Pixel layout is row-major RGBA8, premultiplied alpha.
#[derive(Clone, PartialEq, Eq)]
pub struct Tile {
    pixels: Arc<[u8; TILE_BYTES]>,
}

impl Tile {
    /// A fully transparent (zeroed) tile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pixels: Arc::new([0; TILE_BYTES]),
        }
    }

    /// A tile from raw row-major RGBA8 data.
    #[must_use]
    pub fn from_rgba8(pixels: [u8; TILE_BYTES]) -> Self {
        Self {
            pixels: Arc::new(pixels),
        }
    }

    /// Shared read access to the pixel buffer.
    #[must_use]
    pub fn pixels(&self) -> &[u8; TILE_BYTES] {
        &self.pixels
    }

    /// Mutable access to the pixel buffer; deep-copies first if the buffer
    /// is shared with another tile.
    pub fn pixels_mut(&mut self) -> &mut [u8; TILE_BYTES] {
        Arc::make_mut(&mut self.pixels)
    }

    /// Strong reference count of the shared pixel buffer (test observability
    /// for copy-on-write behavior).
    #[cfg(test)]
    pub(crate) fn shared_count(&self) -> usize {
        Arc::strong_count(&self.pixels)
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tile").finish_non_exhaustive()
    }
}

// Journal persistence moves tiles inside deltas, so `serde` serializes
// the pixel buffer as an opaque byte vector (written explicitly rather
// than derived: the fixed-size array keeps serde out of the hot type).
#[cfg(feature = "serde")]
impl serde::Serialize for Tile {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.pixels())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Tile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let pixels = Vec::<u8>::deserialize(deserializer)?;
        let buffer: [u8; TILE_BYTES] = pixels.clone().try_into().map_err(|_| {
            serde::de::Error::invalid_length(pixels.len(), &"exactly 65536 tile bytes")
        })?;
        Ok(Self::from_rgba8(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tile_is_zeroed() {
        let t = Tile::new();
        assert!(t.pixels().iter().all(|&b| b == 0));
    }

    #[test]
    fn from_rgba8_preserves_data() {
        let mut px = [0u8; TILE_BYTES];
        px[0] = 1;
        px[TILE_BYTES - 1] = 2;
        let t = Tile::from_rgba8(px);
        assert_eq!(t.pixels()[0], 1);
        assert_eq!(t.pixels()[TILE_BYTES - 1], 2);
    }
}
