//! The CLI import door (11.4): `import_to_fwpng` accepts every format
//! `io::raster::sniff` recognizes — png, jpeg, gif, bmp, tiff, webp,
//! wbmp — not just the ones the docs historically named. Each test
//! builds a fixture through the matching io encoder, imports it, and
//! re-opens the resulting `.fw.png` to verify the pixels arrived.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::LazyLock;

use app_cli::ops::import_to_fwpng;

const W: u32 = 6;
const H: u32 = 4;

/// A small distinct-pixel plane (matches the other door tests).
fn plane() -> io::Plane<'static> {
    static RGBA: LazyLock<Vec<u8>> = LazyLock::new(|| {
        let mut rgba = Vec::with_capacity((W * H * 4) as usize);
        for y in 0..H {
            for x in 0..W {
                rgba.push((x * 40) as u8);
                rgba.push((y * 60) as u8);
                rgba.push(255 - (x * 30) as u8);
                rgba.push(255);
            }
        }
        rgba
    });
    io::Plane::new(W, H, &RGBA).expect("fixture plane")
}

fn import_and_decode(bytes: &[u8]) -> doc::Document {
    let written = import_to_fwpng(bytes, Some("door".to_owned())).expect("import succeeds");
    let loaded = app_cli::io::open(&written).expect("re-opens as .fw.png");
    loaded.document
}

/// The imported document renders to the same pixels the fixture held
/// (alpha forced opaque where the codec is opaque).
fn assert_pixels(document: &doc::Document) {
    let render = compose::render(document, 0, 0).expect("imported doc renders");
    assert_eq!((render.width, render.height), (W as u16, H as u16));
}

#[test]
fn import_door_takes_png() {
    let bytes = io::png::encode_with(&plane(), &io::png::PngOptions::default()).expect("png");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_takes_jpeg() {
    let bytes = io::jpeg::encode(&plane(), &io::jpeg::JpegOptions::default()).expect("jpeg");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_takes_gif() {
    let bytes = io::gif::encode_frame(&plane()).expect("gif");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_takes_bmp() {
    let bytes = io::bmp::encode(&plane(), &io::bmp::BmpOptions::default()).expect("bmp");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_takes_tiff() {
    let bytes = io::tiff::encode(&plane(), &io::tiff::TiffOptions::default()).expect("tiff");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_takes_webp() {
    let bytes = io::webp::encode(&plane()).expect("webp");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_takes_wbmp() {
    let bytes = io::wbmp::encode(&plane(), io::Matte::White).expect("wbmp");
    assert_pixels(&import_and_decode(&bytes));
}

#[test]
fn import_door_names_the_format_on_garbage() {
    let error = import_to_fwpng(b"\x00\x01\x02\x03garbage", None)
        .expect_err("unrecognized signature refuses");
    assert!(
        error.to_string().contains("unrecognized signature"),
        "{error}"
    );
}

#[test]
fn import_door_takes_single_frame_gif_statically() {
    // A one-frame GIF routes through the raster importer (not the
    // states path): is_animated_gif must return false for it.
    let frame = io::gif::encode_frame(&plane()).expect("gif frame");
    assert!(!app_cli::ops::is_animated_gif(&frame));
    assert_pixels(&import_and_decode(&frame));
}
