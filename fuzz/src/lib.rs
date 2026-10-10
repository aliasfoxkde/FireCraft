//! Shared fuzz anchors: one function per hostile input surface.
//!
//! The contract for every anchor is "must not panic" — parse errors are
//! expected outcomes and are discarded. Both the stable driver
//! (`src/bin/anchors.rs`) and the libfuzzer targets (`fuzz_targets/`)
//! call these, so a crash found by either is reproducible by the other.

/// Untrusted byte stream interpreted as a PNG chunk sequence.
pub fn anchor_fwpng_chunks(data: &[u8]) {
    let _ = fwpng::parse_chunks(data);
}

/// Untrusted node-tree payload; on success the canonical serializer must
/// also round-trip (parse(serialize(nodes)) parses).
pub fn anchor_fwpng_nodetree(data: &[u8]) {
    if let Ok(nodes) = fwpng::nodetree::parse(data) {
        let round_tripped = fwpng::nodetree::serialize(&nodes);
        let _ = fwpng::nodetree::parse(&round_tripped);
    }
}

/// Untrusted `prVW` thumbnail payload.
pub fn anchor_fwpng_prvw(data: &[u8]) {
    let _ = fwpng::thumbnail::parse_prvw(data);
}

/// Untrusted `mkBT` tile payload.
pub fn anchor_fwpng_mkbt(data: &[u8]) {
    let _ = fwpng::tile::parse_mkbt(data);
}

/// 96+ arbitrary bytes interpreted as two affine transforms (6 f64 each,
/// little endian) and a rect; map_rect and inverse must never panic,
/// whatever bit patterns the bytes hold (NaN, infinities, subnormals).
pub fn anchor_geom_transform(data: &[u8]) {
    let mut vals = [0.0f64; 12];
    for (i, val) in vals.iter_mut().enumerate() {
        let start = i * 8;
        if start + 8 <= data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[start..start + 8]);
            *val = f64::from_le_bytes(bytes);
        }
    }
    let a = geom::Affine::new([vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]]);
    let rect = geom::Rect::new(vals[6], vals[7], vals[8], vals[9]);
    let _ = geom::transform::map_rect(a, rect);
    let _ = a.inverse();
}
