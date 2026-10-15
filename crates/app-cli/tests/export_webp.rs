//! The WebP export door: `export_raster` routes `.webp` through the
//! lossless VP8L codec, and the full CLI export path (render →
//! straight-alpha plane → codec) round-trips a document's pixels.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app_cli::ops::{ExportFlags, export_raster};

/// A one-image document with a distinct pixel pattern (the same shape
/// the raster import path builds).
fn document() -> doc::Document {
    let (width, height) = (7u32, 5u32);
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.push((x * 40) as u8);
            rgba.push((y * 50) as u8);
            rgba.push(255 - (x * 30) as u8);
            rgba.push((200 + (x + y) % 56) as u8);
        }
    }
    io::raster::document_from_image(
        &io::raster::DecodedImage {
            width,
            height,
            rgba,
            source: io::raster::SourceFormat::Png,
        },
        Some("webp door".to_owned()),
    )
    .expect("fixture document")
}

#[test]
fn export_raster_routes_webp_losslessly() {
    let document = document();
    let bytes = export_raster(&document, "out.webp", &ExportFlags::default()).expect("webp export");
    assert!(
        bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "a RIFF/WEBP container: {:?}",
        &bytes[..12]
    );
    // The pixels survive: decode through the import path and compare
    // against the rendered page (one opaque-ish image, no live
    // effects — the render is the image itself).
    let decoded = io::raster::decode(&bytes).expect("webp decode");
    let render = compose::render(&document, 0, 0).expect("fixture renders");
    let straight = compose::straight_alpha(&render.pixels);
    assert_eq!((decoded.width, decoded.height), (7, 5));
    assert_eq!(decoded.rgba, straight);
}

#[test]
fn export_raster_names_the_supported_set_on_unknown_extensions() {
    let document = document();
    let error = export_raster(&document, "out.jxl", &ExportFlags::default())
        .expect_err("unknown extension refuses");
    assert!(
        error.to_string().contains("png|jpg|bmp|tiff|wbmp|webp"),
        "{error}"
    );
}
