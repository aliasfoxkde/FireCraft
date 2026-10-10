#![cfg(target_arch = "wasm32")]
//! Wasm smoke tests for fwpng: the same core paths the native suite
//! covers, executed under `wasm32-unknown-unknown` (node, via
//! `wasm-bindgen-test-runner`). The primary FireCraft release target is
//! a WASM build, so compile-only is not enough — these prove the
//! parsers run there. Inputs are built in memory: `std::fs` does not
//! exist on that target.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use fwpng::chunk::{PNG_SIGNATURE, is_fireworks, parse_chunks, write_chunk};
use std::io::Write as _;
use wasm_bindgen_test::*;

fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(raw).expect("in-memory write");
    enc.finish().expect("in-memory finish")
}

fn synthetic_file(extra: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = PNG_SIGNATURE.to_vec();
    for (ctype, data) in extra {
        write_chunk(&mut out, ctype, data);
    }
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn tile_payload(index: u32, grayscale: u8, raw: &[u8]) -> Vec<u8> {
    let mut chunk = vec![0xFA, 0xCE, 0xCA, 0xFE];
    chunk.extend_from_slice(&(index << 8).to_be_bytes());
    chunk.extend_from_slice(&[0, 0, 0]);
    chunk.push(grayscale);
    chunk.extend_from_slice(&[0u8; 64]);
    chunk.extend_from_slice(&zlib(raw));
    chunk
}

#[wasm_bindgen_test]
fn chunks_parse_and_detect() {
    let file = synthetic_file(&[(b"mkTS", b"MKBv{FRCi{1}}".to_vec())]);
    let chunks = parse_chunks(&file).expect("parses");
    assert_eq!(chunks.len(), 2);
    assert!(is_fireworks(&chunks));
    let plain = synthetic_file(&[]);
    assert!(!is_fireworks(&parse_chunks(&plain).expect("parses")));
}

#[wasm_bindgen_test]
fn nodetree_round_trips() {
    let canonical = b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}}}";
    let nodes = fwpng::nodetree::parse(canonical).expect("parses");
    assert_eq!(fwpng::nodetree::serialize(&nodes), &canonical[..]);
}

#[wasm_bindgen_test]
fn mkbt_tile_decodes() {
    let mut raw = vec![0u8; fwpng::tile::TILE_STREAM_BYTES];
    raw[0] = 0xAA;
    raw[1] = 0x11;
    let chunks = vec![
        fwpng::RawChunk {
            ctype: *b"mkBT",
            data: tile_payload(7, 0, &raw),
            offset: 0,
            crc: 0,
        },
        fwpng::RawChunk {
            ctype: *b"mkBT",
            data: tile_payload(8, 1, &[0x80; fwpng::tile::TILE_STREAM_BYTES]),
            offset: 0,
            crc: 0,
        },
    ];
    let tiles = fwpng::parse_mkbt_chunks(&chunks).expect("tiles decode");
    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].index, 7);
    assert_eq!(tiles[0].to_rgba8()[0..4], [0x11, 0, 0, 0xAA]);
    assert!(tiles[1].grayscale);
}

#[wasm_bindgen_test]
fn prvw_thumbnail_decodes() {
    let mut raw = vec![0xCA, 0xFE, 0xBE, 0xEF];
    raw.extend_from_slice(&64u16.to_be_bytes());
    raw.extend_from_slice(&32u16.to_be_bytes());
    raw.extend_from_slice(&[1, 2, 3]);
    raw.extend_from_slice(&[0, 0, 0]);
    raw.extend_from_slice(&[0u8; fwpng::thumbnail::THUMB_BYTES]);
    let file = synthetic_file(&[(b"prVW", zlib(&raw))]);
    let chunks = parse_chunks(&file).expect("parses");
    let thumb = fwpng::thumbnail::parse_prvw(&chunks[0].data).expect("parses");
    assert_eq!((thumb.width, thumb.height), (64, 32));
    assert_eq!(thumb.palette, vec![[1, 2, 3]]);
}

#[wasm_bindgen_test]
fn document_tree_and_comparison() {
    let tree = b"MKBv{FRCi{1}XLCi{0}}";
    let file = synthetic_file(&[(b"mkTS", zlib(tree)), (b"mkBS", zlib(tree))]);
    let chunks = parse_chunks(&file).expect("parses");
    let current = fwpng::document::parse_mkts(&chunks)
        .expect("mkTS")
        .expect("present");
    let legacy = fwpng::document::parse_mkbs(&chunks)
        .expect("mkBS")
        .expect("present");
    let cmp = fwpng::document::compare_trees(&current, &legacy);
    assert!(cmp.identical);
    assert_eq!(cmp.current_nodes, 3);
}

#[wasm_bindgen_test]
fn standard_png_decodes() {
    let mut pixels = Vec::new();
    for i in 0..8u32 {
        pixels.extend_from_slice(&[i as u8, 0, 0, 255]);
    }
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, 8, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(&pixels).expect("pixels");
    }
    let img = fwpng::decode_standard_rgba8(&encoded).expect("decodes");
    assert_eq!((img.width, img.height), (8, 1));
    assert_eq!(img.rgba8, pixels);
}
