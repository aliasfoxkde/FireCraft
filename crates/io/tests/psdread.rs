//! PSD import battery (M8 task 5).
//!
//! Every fixture is hand-built from the public Adobe spec — the house
//! byte-array style — plus one real-writer oracle fixture generated
//! with `ag-psd` (MIT, PLANNING §7.6). The battery pins: flat and
//! layered documents, raw and PackBits channels, the group pair,
//! Unicode names, opacity/blend/visibility mapping, every report-note
//! divergence (masks, clipping, text, adjustment, unknown blends —
//! including ImageMagick's byte-reversed-key bug), and every refuse
//! gate. Nothing panics.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::ObjectData;
use io::psd;

// ---------------------------------------------------------------- //
// Fixture builder: the PSD spec, in test-local helpers.
// ---------------------------------------------------------------- //

fn be16(value: u16) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

fn be32(value: u32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

fn be32s(value: i32) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

fn section(body: &[u8]) -> Vec<u8> {
    let mut out = be32(body.len() as u32);
    out.extend_from_slice(body);
    out
}

/// The 26-byte header: `8BPS` v1, `channels` channels, `width`×
/// `height`, `depth`-bit, `mode` color mode.
fn header(width: u32, height: u32, channels: u16, depth: u16, mode: u16) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&be16(1));
    out.extend_from_slice(&[0; 6]);
    out.extend_from_slice(&be16(channels));
    out.extend_from_slice(&be32(height));
    out.extend_from_slice(&be32(width));
    out.extend_from_slice(&be16(depth));
    out.extend_from_slice(&be16(mode));
    out
}

/// One channel-info entry (id + data length) and its stream.
struct Channel {
    id: i16,
    /// Compression marker; `None` for a zero-length stream.
    compression: Option<u16>,
    rows: Vec<Vec<u8>>,
}

impl Channel {
    fn raw(id: i16, rows: Vec<Vec<u8>>) -> Self {
        Self {
            id,
            compression: Some(0),
            rows,
        }
    }

    fn rle(id: i16, rows: Vec<Vec<u8>>) -> Self {
        Self {
            id,
            compression: Some(1),
            rows,
        }
    }

    fn stream_bytes(&self) -> Vec<u8> {
        let Some(compression) = self.compression else {
            return Vec::new();
        };
        let mut out = be16(compression);
        match compression {
            0 => {
                for row in &self.rows {
                    out.extend_from_slice(row);
                }
            }
            _ => {
                let mut counts = Vec::new();
                let mut packed = Vec::new();
                for row in &self.rows {
                    counts.extend_from_slice(&be16(pack_row(row).len() as u16));
                    packed.extend_from_slice(&pack_row(row));
                }
                out.extend_from_slice(&counts);
                out.extend_from_slice(&packed);
            }
        }
        out
    }
}

/// PackBits-encodes one scanline (run-length for repeats; the test
/// pixels are uniform runs, so this encoder is exact for them).
fn pack_row(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < row.len() {
        let value = row[at];
        let mut run = 1;
        while at + run < row.len() && row[at + run] == value && run < 128 {
            run += 1;
        }
        out.push((1 - run as i8) as u8);
        out.push(value);
        at += run;
    }
    out
}

/// A hand-built layer record.
struct LayerFixture {
    top: i32,
    left: i32,
    width: u32,
    height: u32,
    channels: Vec<Channel>,
    blend: [u8; 4],
    opacity: u8,
    clipping: u8,
    flags: u8,
    pascal_name: String,
    /// Additional-info blocks (key + body), assembled verbatim.
    info_blocks: Vec<(&'static [u8; 4], Vec<u8>)>,
}

impl LayerFixture {
    fn raster(name: &str, top: i32, left: i32, width: u32, height: u32, rgba: &[u8]) -> Self {
        let planes = |offset: usize| -> Vec<Vec<u8>> {
            (0..height as usize)
                .map(|y| {
                    (0..width as usize)
                        .map(|x| rgba[(y * width as usize + x) * 4 + offset])
                        .collect()
                })
                .collect()
        };
        Self {
            top,
            left,
            width,
            height,
            channels: vec![
                Channel::raw(0, planes(0)),
                Channel::raw(1, planes(1)),
                Channel::raw(2, planes(2)),
                Channel::raw(-1, planes(3)),
            ],
            blend: *b"norm",
            opacity: 255,
            clipping: 0,
            flags: 0,
            pascal_name: name.to_owned(),
            info_blocks: Vec::new(),
        }
    }

    fn group_open(name: &str, blend: [u8; 4], opacity: u8) -> Self {
        Self {
            top: 0,
            left: 0,
            width: 0,
            height: 0,
            channels: Vec::new(),
            blend,
            opacity,
            clipping: 0,
            flags: 0,
            pascal_name: name.to_owned(),
            info_blocks: vec![(b"lsct", be32s(1))],
        }
    }

    fn group_close() -> Self {
        Self {
            top: 0,
            left: 0,
            width: 0,
            height: 0,
            channels: Vec::new(),
            blend: *b"norm",
            opacity: 255,
            clipping: 0,
            flags: 0,
            pascal_name: "</Layer set>".to_owned(),
            info_blocks: vec![(b"lsct", be32s(3))],
        }
    }

    fn pascal(name: &str) -> Vec<u8> {
        let mut out = vec![name.len() as u8];
        out.extend_from_slice(name.as_bytes());
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&be32s(self.top));
        out.extend_from_slice(&be32s(self.left));
        out.extend_from_slice(&be32s(self.top + self.height as i32));
        out.extend_from_slice(&be32s(self.left + self.width as i32));
        out.extend_from_slice(&be16(self.channels.len() as u16));
        // Channel lengths are filled in a second pass once the streams
        // (and the extra data) are sized.
        let mut streams = Vec::new();
        for channel in &self.channels {
            streams.extend_from_slice(&channel.stream_bytes());
        }
        for channel in &self.channels {
            out.extend_from_slice(&be16s(channel.id));
            let length = if channel.compression.is_some() {
                channel.stream_bytes().len()
            } else {
                0
            };
            out.extend_from_slice(&be32(length as u32));
        }
        out.extend_from_slice(b"8BIM");
        out.extend_from_slice(&self.blend);
        out.push(self.opacity);
        out.push(self.clipping);
        out.push(self.flags);
        out.push(0); // filler
        // Extra data: mask (0), ranges (0), pascal name, info blocks.
        let mut extra = be32(0);
        extra.extend_from_slice(&be32(0));
        extra.extend_from_slice(&Self::pascal(&self.pascal_name));
        for (key, body) in &self.info_blocks {
            extra.extend_from_slice(b"8BIM");
            extra.extend_from_slice(*key);
            extra.extend_from_slice(&section(body));
            while extra.len() % 4 != 0 {
                extra.push(0);
            }
        }
        out.extend_from_slice(&section(&extra));
        out.extend_from_slice(&streams);
        out
    }
}

fn be16s(value: i16) -> Vec<u8> {
    value.to_be_bytes().to_vec()
}

/// Assembles a full PSD: header, empty color mode + resources, the
/// layer section over `layers` (streams interleaved after each
/// record), then the raw plane-major composite.
fn build_psd(
    width: u32,
    height: u32,
    layers: Vec<LayerFixture>,
    composite: Option<&[u8]>,
) -> Vec<u8> {
    let mut out = header(width, height, 3, 8, 3);
    out.extend_from_slice(&section(&[])); // color mode data
    out.extend_from_slice(&section(&[])); // image resources

    let mut info = be16(layers.len() as u16);
    for layer in &layers {
        info.extend_from_slice(&layer.encode());
    }
    // The layer-and-mask section wraps the layer-info section.
    out.extend_from_slice(&section(&section(&info)));

    // Merged composite: compression 0, then the R, G, B planes.
    let mut merged = be16(0);
    match composite {
        Some(planes) => merged.extend_from_slice(planes),
        None => merged.extend(std::iter::repeat_n(
            0u8,
            width as usize * height as usize * 3,
        )),
    }
    out.extend_from_slice(&section(&merged));
    out
}

fn pixels(width: u32, height: u32, seed: u8) -> Vec<u8> {
    (0..(width * height * 4) as usize)
        .map(|index| (index as u8).wrapping_add(seed))
        .collect()
}

// ---------------------------------------------------------------- //
// The battery.
// ---------------------------------------------------------------- //

#[test]
fn flat_composite_only_document() {
    // 2×3, no layers: the merged composite becomes one IMGv document.
    // Plane-major: R rows, G rows, B rows — pixel (0,0) red, (1,0)
    // green, (0,1) blue, (1,2) cyan.
    let composite: &[u8] = &[
        255, 0, 0, 255, 0, 0, // R
        0, 255, 0, 0, 0, 255, // G
        0, 0, 255, 0, 0, 255, // B
    ];
    let bytes = build_psd(2, 3, Vec::new(), Some(composite));
    let imported = psd::decode(&bytes).expect("flat psd imports");
    assert!(imported.composite);
    assert_eq!(imported.layers, 0);
    let document = &imported.document;
    assert_eq!(document.info.canvas.width, 2.0);
    assert_eq!(document.info.canvas.height, 3.0);
    let page = &document.pages[0];
    assert_eq!(page.layers.layers.len(), 1);
    let cell = &page.layers.layers[0].cells[0];
    assert_eq!(cell.elements.len(), 1);
    assert!(matches!(
        document.registry.get(cell.elements[0]),
        Some(ObjectData::Image(_))
    ));
    // Tile pixels are addressed (x + y*128): the 2-wide image sits in
    // the left edge of the tile, so row 1 starts at offset 128.
    let tile = document.tiles.get(0).expect("tile bank loaded");
    let px = |x: usize, y: usize| {
        let at = (y * 128 + x) * 4;
        &tile.pixels()[at..at + 4]
    };
    assert_eq!(px(0, 0), &[255, 0, 0, 255]); // red
    assert_eq!(px(1, 0), &[0, 255, 0, 255]); // green
    assert_eq!(px(0, 1), &[0, 0, 255, 255]); // blue
    assert_eq!(px(1, 2), &[0, 255, 255, 255]); // cyan
}

#[test]
fn rle_composite_shares_one_byte_count_table() {
    // The composite's image-data section pools every channel's u16
    // byte counts into ONE table (rows x channels), then packs all
    // rows into one stream — how ImageMagick, ag-psd, and Photoshop
    // write it. Per-channel tables would misparse every real flat
    // RLE file (found against an ImageMagick sample).
    // R rows [255,255] [0,0]; G rows [0,255] x2; B rows [0,0] [255,255]
    // -> red, yellow, blue, white.
    let mut merged = be16(1); // compression: RLE
    let rows: [[u8; 2]; 6] = [
        [255, 255],
        [0, 0], // R: red row / black row
        [0, 255],
        [0, 255], // G: both green
        [0, 0],
        [255, 255], // B: black row / blue row
    ];
    // All six byte-count entries first (two per channel), sized from
    // the packed rows...
    let packed: Vec<Vec<u8>> = rows.iter().map(|row| pack_row(row)).collect();
    for row in &packed {
        merged.extend_from_slice(&be16(row.len() as u16));
    }
    // ...then one packed stream: R rows, then G rows, then B rows.
    for row in &packed {
        merged.extend_from_slice(row);
    }
    let mut bytes = header(2, 2, 3, 8, 3);
    bytes.extend_from_slice(&section(&[])); // color mode data
    bytes.extend_from_slice(&section(&[])); // image resources
    bytes.extend_from_slice(&be32(0)); // no layer section
    bytes.extend_from_slice(&section(&merged));

    let imported = psd::decode(&bytes).expect("rle composite imports");
    assert!(imported.composite);
    assert_eq!(imported.layers, 0);
    let document = &imported.document;
    let cell = &document.pages[0].layers.layers[0].cells[0];
    assert!(matches!(
        document.registry.get(cell.elements[0]),
        Some(ObjectData::Image(_))
    ));
    // The flat composite's image is the bank's first object: its tile
    // is slot 0, addressed (x + y*128).
    let tile = document.tiles.get(0).expect("composite tile exists");
    let px = |x: usize, y: usize| {
        let at = (y * 128 + x) * 4;
        &tile.pixels()[at..at + 4]
    };
    assert_eq!(px(0, 0), &[255, 0, 0, 255]); // red
    assert_eq!(px(1, 0), &[255, 255, 0, 255]); // yellow
    assert_eq!(px(0, 1), &[0, 0, 255, 255]); // blue
    assert_eq!(px(1, 1), &[0, 255, 255, 255]); // cyan
}

#[test]
fn layered_document_maps_layers_and_offsets() {
    let red = vec![
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    let blue = vec![
        0, 0, 255, 128, 0, 0, 255, 128, 0, 0, 255, 128, 0, 0, 255, 128,
    ];
    let bytes = build_psd(
        4,
        4,
        vec![
            LayerFixture::raster("bottom", 0, 0, 2, 2, &red),
            LayerFixture::raster("top", 1, 1, 2, 2, &blue),
        ],
        None,
    );
    let imported = psd::decode(&bytes).expect("layered psd imports");
    assert_eq!(imported.layers, 2);
    let document = &imported.document;
    let page = &document.pages[0];
    assert_eq!(page.layers.layers.len(), 2);
    assert_eq!(page.layers.layers[0].name, "bottom");
    assert_eq!(page.layers.layers[1].name, "top");
    for (index, layer) in page.layers.layers.iter().enumerate() {
        let Some(ObjectData::Image(image)) = document.registry.get(layer.cells[0].elements[0])
        else {
            panic!("layer {index} holds an image object");
        };
        let (x, y) = if index == 0 { (0.0, 0.0) } else { (1.0, 1.0) };
        assert_eq!(image.location, geom::Point::new(x, y));
        assert_eq!(image.opacity, 1000);
        assert!(image.visible);
    }
    // The semi-transparent top layer's tile is premultiplied: alpha
    // 128 blue arrives as b = (255*128+127)/255 = 128.
    let tile = document.tiles.get(1).expect("second layer's tile");
    assert_eq!(tile.pixels()[0..4], [0, 0, 128, 128]);
}

#[test]
fn rle_channels_decode_exactly() {
    let mut layer = LayerFixture::raster("packed", 0, 0, 3, 2, &pixels(3, 2, 7));
    layer.channels = vec![
        Channel::rle(0, vec![vec![1, 2, 3], vec![4, 5, 6]]),
        Channel::rle(1, vec![vec![7, 8, 9], vec![10, 11, 12]]),
        Channel::rle(2, vec![vec![13, 14, 15], vec![16, 17, 18]]),
        Channel::rle(-1, vec![vec![255; 3], vec![255; 3]]),
    ];
    let bytes = build_psd(3, 2, vec![layer], None);
    let imported = psd::decode(&bytes).expect("rle psd imports");
    assert_eq!(imported.layers, 1);
    let tile = imported.document.tiles.get(0).expect("tile");
    let got = tile.pixels();
    assert_eq!(got[0..4], [1, 7, 13, 255]); // (0,0)
    assert_eq!(got[4..8], [2, 8, 14, 255]); // (1,0)
    // Row 1 starts at tile offset 128.
    assert_eq!(got[512..516], [4, 10, 16, 255]); // (0,1)
}

#[test]
fn group_pair_becomes_structured_group_object() {
    let red = vec![
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    // Bottom-first: the bounding divider, the member, then the group
    // header (which carries the group's semantics), then topmost.
    let bytes = build_psd(
        2,
        2,
        vec![
            LayerFixture::group_close(),
            LayerFixture::raster("member", 0, 0, 2, 1, &red),
            LayerFixture::group_open("the set", *b"mul ", 128),
            LayerFixture::raster("outside", 0, 0, 2, 1, &red),
        ],
        None,
    );
    let imported = psd::decode(&bytes).expect("grouped psd imports");
    assert_eq!(imported.groups, 1);
    assert_eq!(imported.layers, 2);
    let document = &imported.document;
    let page = &document.pages[0];
    assert_eq!(page.layers.layers.len(), 2);
    // The group layer's cell holds the GroupObject.
    let group_cell = &page.layers.layers[0].cells[0];
    assert_eq!(group_cell.elements.len(), 1);
    let Some(ObjectData::Group(group)) = document.registry.get(group_cell.elements[0]) else {
        panic!("group pair imports as a group object");
    };
    assert_eq!(group.opacity, 502); // 128/255 → 502‰
    assert_eq!(group.blend, 2); // multiply
    assert_eq!(group.children.len(), 1);
    assert!(matches!(
        document.registry.get(group.children[0]),
        Some(ObjectData::Image(_))
    ));
    // The terminator record is not a layer of its own.
    assert_eq!(page.layers.layers[1].name, "outside");
}

#[test]
fn unicode_name_wins_over_pascal() {
    let mut layer = LayerFixture::raster("pascal", 0, 0, 1, 1, &[1, 2, 3, 255]);
    // luni: u32 count + UTF-16BE "Ünïcode".
    let text: Vec<u16> = "Ünïcode".encode_utf16().collect();
    let mut body = be32(text.len() as u32);
    for unit in text {
        body.extend_from_slice(&be16(unit));
    }
    layer.info_blocks.push((b"luni", body));
    let bytes = build_psd(1, 1, vec![layer], None);
    let imported = psd::decode(&bytes).expect("luni psd imports");
    assert_eq!(imported.document.pages[0].layers.layers[0].name, "Ünïcode");
}

#[test]
fn hidden_layer_is_invisible() {
    let mut layer = LayerFixture::raster("ghost", 0, 0, 1, 1, &[1, 2, 3, 255]);
    layer.flags = 0x02;
    let bytes = build_psd(1, 1, vec![layer], None);
    let imported = psd::decode(&bytes).expect("hidden psd imports");
    let page = &imported.document.pages[0];
    assert!(!page.layers.layers[0].display);
    let id = page.layers.layers[0].cells[0].elements[0];
    let Some(ObjectData::Image(image)) = imported.document.registry.get(id) else {
        panic!("hidden layer still imports");
    };
    assert!(!image.visible);
}

#[test]
fn text_layer_flattens_with_a_note() {
    let mut layer = LayerFixture::raster("headline", 0, 0, 1, 1, &[1, 2, 3, 255]);
    layer.info_blocks.push((b"TySh", vec![0; 16]));
    let bytes = build_psd(1, 1, vec![layer], None);
    let imported = psd::decode(&bytes).expect("text psd imports");
    assert_eq!(imported.texts_as_raster, 1);
    assert!(
        imported
            .notes
            .iter()
            .any(|note| note.contains("text layer") && note.contains("raster"))
    );
    assert_eq!(imported.layers, 1);
}

#[test]
fn adjustment_layer_is_skipped_with_a_note() {
    let mut layer = LayerFixture::raster("brightness", 0, 0, 1, 1, &[1, 2, 3, 255]);
    layer.info_blocks.push((b"brit", vec![0; 12]));
    let bytes = build_psd(1, 1, vec![layer], None);
    let imported = psd::decode(&bytes).expect("adjustment psd imports");
    assert_eq!(imported.layers, 0);
    assert!(
        imported
            .notes
            .iter()
            .any(|note| note.contains("adjustment layer"))
    );
    // The page still exists (empty layer list).
    assert_eq!(imported.document.pages[0].layers.layers.len(), 0);
}

#[test]
fn reversed_blend_key_is_a_note_not_an_error() {
    // ImageMagick 7.1.1-43 writes 'mron' (norm byte-reversed) on one
    // of its writer paths; decision 0006 pins tolerance.
    let mut layer = LayerFixture::raster("from-im", 0, 0, 1, 1, &[1, 2, 3, 255]);
    layer.blend = *b"mron";
    let bytes = build_psd(1, 1, vec![layer], None);
    let imported = psd::decode(&bytes).expect("reversed key imports");
    assert_eq!(imported.unknown_blends, 1);
    let id = imported.document.pages[0].layers.layers[0].cells[0].elements[0];
    let Some(ObjectData::Image(image)) = imported.document.registry.get(id) else {
        panic!("layer imports");
    };
    assert_eq!(image.blend, 0);
    assert!(
        imported
            .notes
            .iter()
            .any(|note| note.contains("blend mode \"mron\""))
    );
}

#[test]
fn clipped_layer_and_masks_report_but_import() {
    let mut layer = LayerFixture::raster("clipped", 0, 0, 1, 1, &[1, 2, 3, 255]);
    layer.clipping = 1;
    layer
        .channels
        .push(Channel::raw(-2, vec![vec![255], vec![255]])); // layer mask
    let bytes = build_psd(1, 1, vec![layer], None);
    let imported = psd::decode(&bytes).expect("clipped psd imports");
    assert_eq!(imported.clipped_layers, 1);
    assert_eq!(imported.masks_dropped, 1);
    assert_eq!(imported.layers, 1);
}

#[test]
fn blend_table_maps_all_model_modes() {
    let cases: &[([u8; 4], i64)] = &[
        (*b"norm", 0),
        (*b"mul ", 2),
        (*b"diff", 3),
        (*b"over", 4),
        (*b"scrn", 5),
        (*b"dark", 9),
        (*b"lite", 10),
        (*b"hue ", 11),
        (*b"sat ", 12),
        (*b"colr", 13),
        (*b"lum ", 14),
        (*b"smud", 32),
        (*b"hLit", 33),
        (*b"sLit", 34),
        (*b"div ", 36),
        (*b"idiv", 37),
    ];
    for (key, want) in cases {
        let mut layer = LayerFixture::raster("blendy", 0, 0, 1, 1, &[1, 2, 3, 255]);
        layer.blend = *key;
        let bytes = build_psd(1, 1, vec![layer], None);
        let imported = psd::decode(&bytes).expect("blend psd imports");
        assert_eq!(imported.unknown_blends, 0, "key {key:?} should map");
        let id = imported.document.pages[0].layers.layers[0].cells[0].elements[0];
        let Some(ObjectData::Image(image)) = imported.document.registry.get(id) else {
            panic!("layer imports");
        };
        assert_eq!(image.blend, *want, "key {key:?}");
    }
}

#[test]
fn ag_psd_oracle_fixture_parses_to_the_expected_tree() {
    // Generated with ag-psd (MIT oracle, decision 0006): backdrop +
    // group(inner-a multiply, inner-b screen, ghost hidden) + topmost
    // overlay @ 128.
    let bytes = include_bytes!("fixtures/ag_group.psd");
    let imported = psd::decode(bytes).expect("ag-psd oracle imports");
    let document = &imported.document;
    let page = &document.pages[0];
    assert_eq!(page.layers.layers.len(), 3);
    assert_eq!(page.layers.layers[0].name, "backdrop");
    assert_eq!(page.layers.layers[2].name, "topmost");
    // The middle layer is the group (named by its header record).
    let group_cell = &page.layers.layers[1].cells[0];
    let Some(ObjectData::Group(group)) = document.registry.get(group_cell.elements[0]) else {
        panic!("the oracle's <Layer set> imports as a group object");
    };
    assert_eq!(group.children.len(), 3);
    assert_eq!(imported.groups, 1);
    // backdrop, inner-a, inner-b, ghost, topmost — five rasters.
    assert_eq!(imported.layers, 5);
    // Pass-through (0.6 opacity → 153/255 → 600‰) lands on normal.
    assert_eq!(group.opacity, 600);
    assert_eq!(group.blend, 0);
    // (ag-psd's writer never emits the clipping byte, so the clipped
    // member arrives unclipped; the hand-built fixture pins clipping.)
    // The group pair walked cleanly — no pair diagnostics.
    assert!(
        !imported
            .notes
            .iter()
            .any(|note| note.contains("never terminated") || note.contains("no bounding divider"))
    );
    // topmost carried opacity 128 → 502‰ over overlay.
    let top_id = page.layers.layers[2].cells[0].elements[0];
    let Some(ObjectData::Image(top)) = document.registry.get(top_id) else {
        panic!("topmost imports");
    };
    assert_eq!(top.opacity, 502);
    assert_eq!(top.blend, 4);
}

#[test]
fn hostile_streams_refuse_without_panicking() {
    let good = build_psd(
        2,
        2,
        vec![LayerFixture::raster("a", 0, 0, 1, 1, &[1, 2, 3, 255])],
        None,
    );
    // Truncation at every prefix refuses — except inside the merged
    // composite (the last 18 bytes: u32 length + marker + 2×2 RGB),
    // where the remaining bytes are simultaneously a valid length-less
    // composite; an accept there is benign as long as the layer
    // document is intact (the composite is validation-only).
    let composite_at = good.len() - 18;
    for cut in 1..good.len() {
        let result = psd::decode(&good[..cut]);
        if cut <= composite_at {
            assert!(result.is_err(), "prefix of {cut} bytes must refuse");
        } else if let Ok(imported) = result {
            assert_eq!(imported.layers, 1, "cut {cut} kept the layer");
        }
    }
    // Bad signature.
    let mut bad = good.clone();
    bad[0] = b'X';
    assert!(psd::decode(&bad).is_err());
    // PSB (version 2).
    let mut psb = header(2, 2, 3, 8, 3);
    psb[4] = 0;
    psb[5] = 2;
    assert!(matches!(
        psd::decode(&psb),
        Err(io::ImportError::Unsupported { .. })
    ));
    // 16-bit depth (ImageMagick's default).
    let deep = header(2, 2, 3, 16, 3);
    assert!(matches!(
        psd::decode(&deep),
        Err(io::ImportError::Unsupported { reason, .. }) if reason.contains("16-bit")
    ));
    // Grayscale mode.
    let gray = header(2, 2, 1, 8, 1);
    assert!(matches!(
        psd::decode(&gray),
        Err(io::ImportError::Unsupported { reason, .. }) if reason.contains("grayscale")
    ));
    // Zero canvas.
    let empty = header(0, 0, 3, 8, 3);
    assert!(matches!(
        psd::decode(&empty),
        Err(io::ImportError::Unsupported { .. })
    ));
    // Canvas over the compose ceiling.
    let huge = header(65536, 1, 3, 8, 3);
    assert!(matches!(
        psd::decode(&huge),
        Err(io::ImportError::Unsupported { reason, .. }) if reason.contains("65535")
    ));
    // ZIP channel compression refuses with its reason.
    let mut zipped = LayerFixture::raster("z", 0, 0, 1, 1, &[1, 2, 3, 255]);
    zipped.channels[0].compression = Some(2);
    let zip_bytes = build_psd(1, 1, vec![zipped], None);
    assert!(matches!(
        psd::decode(&zip_bytes),
        Err(io::ImportError::Unsupported { reason, .. }) if reason.contains("ZIP")
    ));
    // An absurd channel count.
    let mut absurd = LayerFixture::raster("many", 0, 0, 1, 1, &[1, 2, 3, 255]);
    absurd.channels = (0..70)
        .map(|index| Channel::raw(index as i16, vec![vec![0]]))
        .collect();
    let absurd_bytes = build_psd(1, 1, vec![absurd], None);
    assert!(psd::decode(&absurd_bytes).is_err());
    // A mutated 8BIM signature.
    let mut sig = build_psd(
        2,
        2,
        vec![LayerFixture::raster("a", 0, 0, 1, 1, &[1, 2, 3, 255])],
        None,
    );
    let at = sig.windows(4).position(|w| w == b"8BIM").expect("8BIM");
    sig[at] = b'X';
    assert!(psd::decode(&sig).is_err());
}

#[test]
fn negative_layer_count_notes_merged_alpha() {
    let mut bytes = build_psd(
        2,
        2,
        vec![LayerFixture::raster("only", 0, 0, 1, 1, &[1, 2, 3, 255])],
        None,
    );
    // Flip the layer count negative — it sits after both section
    // lengths (26-byte header + 4 + 4 + the two u32 prefixes).
    let count_at = 26 + 4 + 4 + 4 + 4;
    bytes[count_at..count_at + 2].copy_from_slice(&(-1i16).to_be_bytes());
    let imported = psd::decode(&bytes).expect("negative count imports");
    assert!(
        imported
            .notes
            .iter()
            .any(|note| note.contains("negative layer count"))
    );
    assert_eq!(imported.layers, 1);
}
