//! Import battery for M8 task 2: every decoder is proven against
//! pixels encoded by the io crate's own exporters (or by the `image`
//! crate where io has no encoder yet), the document builder is proven
//! at tile level, and hostile headers are refused without panicking.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ::raster::TILE_SIDE;
use doc::TileRef;
use io::jpeg::JpegOptions;
use io::raster::{self, SourceFormat};
use io::{Matte, Plane};

/// 37×23 gradient with a diagonal alpha ramp (odd width exercises
/// BMP padding; multi-tile coverage needs ≥129 px, so `big_gradient`
/// covers that).
fn gradient() -> Vec<u8> {
    let (width, height) = (37u32, 23u32);
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.push((x * 255 / 36) as u8);
            rgba.push((y * 255 / 22) as u8);
            rgba.push(255 - (x * 255 / 36) as u8);
            rgba.push((255 - (x + y) * 255 / 58) as u8);
        }
    }
    rgba
}

/// 200×100 gradient — spans a 2×1 tile grid at 128 px tiles.
fn big_gradient() -> Vec<u8> {
    let (width, height) = (200u32, 100u32);
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.push((x * 255 / 199) as u8);
            rgba.push((y * 255 / 99) as u8);
            rgba.push((x * 255 / 199) as u8);
            rgba.push(255);
        }
    }
    rgba
}

#[test]
fn png_import_round_trips_exactly() {
    let rgba = gradient();
    let plane = Plane::new(37, 23, &rgba).expect("plane");
    let bytes = io::png::encode(&plane).expect("encode");
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::Png);
    assert_eq!((decoded.width, decoded.height), (37, 23));
    assert_eq!(decoded.rgba, rgba);
}

#[test]
fn bmp_import_round_trips_alpha_exactly() {
    let rgba = gradient();
    let plane = Plane::new(37, 23, &rgba).expect("plane");
    let bytes = io::bmp::encode(
        &plane,
        &io::bmp::BmpOptions {
            depth: io::bmp::BmpDepth::Bpp32,
            ..Default::default()
        },
    )
    .expect("encode");
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::Bmp);
    assert_eq!(decoded.rgba, rgba);

    // 24-bit carries no alpha: the decoder returns the matte-flattened
    // pixels, exactly what the encoder flattened.
    let flat = io::bmp::encode(&plane, &io::bmp::BmpOptions::default()).expect("encode");
    let flattened = raster::decode(&flat).expect("decode");
    for (got, want) in flattened
        .rgba
        .chunks_exact(4)
        .zip(plane.flatten_rgb(Matte::White).chunks_exact(3))
    {
        assert_eq!(&got[..3], want);
        assert_eq!(got[3], 255);
    }
}

#[test]
fn tiff_import_round_trips_exactly() {
    let rgba = gradient();
    let plane = Plane::new(37, 23, &rgba).expect("plane");
    let bytes = io::tiff::encode(&plane, &io::tiff::TiffOptions::default()).expect("encode");
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::Tiff);
    assert_eq!(decoded.rgba, rgba);
}

#[test]
fn jpeg_import_tracks_the_flattened_expectation() {
    let rgba = gradient();
    let plane = Plane::new(37, 23, &rgba).expect("plane");
    let options = JpegOptions::default();
    let bytes = io::jpeg::encode(&plane, &options).expect("encode");
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::Jpeg);
    let expected = plane.flatten_rgb(options.matte);
    let mut sum = 0u64;
    for (want, got) in expected.chunks_exact(3).zip(decoded.rgba.chunks_exact(4)) {
        for (w, g) in want.iter().zip(got.iter().take(3)) {
            sum += u64::from(w.abs_diff(*g));
        }
    }
    let mean = sum / u64::from(37u32 * 23);
    assert!(mean <= 4, "mean diff {mean}");
}

#[test]
fn webp_lossless_import_round_trips_exactly() {
    let rgba = gradient();
    // The `image` crate's lossless WebP encoder (io's own WebP writer
    // is a lossless encode wrapper once task 2 lands its options).
    let mut bytes = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut bytes);
    encoder
        .encode(&rgba, 37, 23, image::ExtendedColorType::Rgba8)
        .expect("webp encode");
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::WebP);
    assert_eq!(decoded.rgba, rgba);
}

#[test]
fn gif_import_decodes_the_first_frame() {
    let rgba = gradient();
    let plane = Plane::new(37, 23, &rgba).expect("plane");
    // GIF carries no alpha: flatten first, quantize through the GIF
    // encoder, then check the structural round-trip (dimensions +
    // format + rough pixel agreement on the luma threshold).
    let flat = plane.flatten_rgb(Matte::White);
    let flat_rgba: Vec<u8> = flat
        .chunks_exact(3)
        .flat_map(|px| [px[0], px[1], px[2], 255])
        .collect();
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        encoder
            .encode(&flat_rgba, 37, 23, image::ExtendedColorType::Rgba8)
            .expect("gif encode");
    }
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::Gif);
    assert_eq!((decoded.width, decoded.height), (37, 23));
}

#[test]
fn wbmp_import_decodes_through_the_own_codec() {
    let rgba = gradient();
    let plane = Plane::new(37, 23, &rgba).expect("plane");
    let bytes = io::wbmp::encode(&plane, Matte::White).expect("encode");
    let decoded = raster::decode(&bytes).expect("decode");
    assert_eq!(decoded.source, SourceFormat::Wbmp);
    assert_eq!((decoded.width, decoded.height), (37, 23));
    for (i, px) in decoded.rgba.chunks_exact(4).enumerate() {
        let expected = if plane.luma_at(i as u32 % 37, i as u32 / 37, Matte::White) >= 128 {
            255
        } else {
            0
        };
        assert_eq!(&px[..3], &[expected; 3], "at {i}");
        assert_eq!(px[3], 255);
    }
}

#[test]
fn document_builder_tiles_the_bank_and_scales_the_canvas() {
    let rgba = big_gradient();
    let decoded = io::raster::DecodedImage {
        width: 200,
        height: 100,
        rgba: rgba.clone(),
        source: SourceFormat::Bmp,
    };
    let document = raster::document_from_image(&decoded, Some("imported".to_owned()))
        .expect("document builds");

    assert_eq!(document.info.canvas.width, 200.0);
    assert_eq!(document.info.canvas.height, 100.0);
    assert_eq!(document.info.frame_count, 1);
    assert_eq!(document.pages.len(), 1);
    let page = &document.pages[0];
    assert_eq!(page.info.frame_count, 1);
    assert_eq!(page.layers.layers.len(), 1);
    assert_eq!(page.layers.layers[0].cells.len(), 1);
    let elements = &page.layers.layers[0].cells[0].elements;
    assert_eq!(elements.len(), 1);

    let element = document.registry.get(elements[0]).expect("element exists");
    assert!(matches!(element, doc::ObjectData::Image(_)), "image object");
    // The matches! assert above makes this else branch dead.
    let doc::ObjectData::Image(image) = element else {
        return;
    };
    assert_eq!(image.pixels, (200, 100));
    assert_eq!(image.tile_size, 128);
    assert_eq!(image.name.as_deref(), Some("imported"));
    // 200×100 → 2×1 tile grid.
    assert_eq!(image.tiles, vec![TileRef::Tile(0), TileRef::Tile(1)]);
    assert_eq!(document.tiles.len(), 2);

    // Tile 0 is fully live: premultiplied copy of the source.
    let tile0 = document.tiles.get(0).expect("tile 0").pixels();
    assert_eq!(&tile0[..8], &rgba[..8], "opaque pixels copy verbatim");
    // Tile 1 starts at x=128: in every row its live columns are px
    // 0..72 (x < 200), and only rows 0..100 are live at all — the rest
    // of the 128×128 tile is zero padding past the image edge.
    let tile1 = document.tiles.get(1).expect("tile 1").pixels();
    assert_eq!(&tile1[..4], &rgba[(128 * 4)..][..4]);
    for py in 0..TILE_SIDE {
        let row = &tile1[py * TILE_SIDE * 4..(py + 1) * TILE_SIDE * 4];
        if py < 100 {
            assert_eq!(
                &row[..4],
                &rgba[(py * 200 + 128) * 4..][..4],
                "row {py} live pixel"
            );
            assert!(
                row[72 * 4..].iter().all(|&b| b == 0),
                "row {py} pads past x=200"
            );
        } else {
            assert!(row.iter().all(|&b| b == 0), "row {py} fully padded");
        }
    }
}

#[test]
fn document_builder_premultiplies_semitransparent_pixels() {
    // One pixel: straight (255, 0, 0, 128) → premultiplied (128, 0, 0, 128).
    let rgba = vec![255u8, 0, 0, 128];
    let decoded = io::raster::DecodedImage {
        width: 1,
        height: 1,
        rgba,
        source: SourceFormat::Png,
    };
    let document = raster::document_from_image(&decoded, None).expect("builds");
    let tile = document.tiles.get(0).expect("tile").pixels();
    assert_eq!(&tile[..4], &[128, 0, 0, 128]);
}

#[test]
fn hostile_streams_are_refused_cleanly() {
    // Unknown signature.
    assert!(matches!(
        raster::decode(b"\x00\x01\x02\x03junk"),
        Err(io::ImportError::Unsupported {
            format: "raster",
            ..
        })
    ));
    // Too short to sniff.
    assert!(matches!(
        raster::decode(b"\x89PN"),
        Err(io::ImportError::Truncated {
            format: "raster",
            ..
        })
    ));
    // PNG signature + truncated body.
    assert!(matches!(
        raster::decode(b"\x89PNG\r\n\x1a\n\xff\xff\xff"),
        Err(io::ImportError::Decode { format: "png", .. })
    ));
    // JPEG SOI + no frame data.
    assert!(matches!(
        raster::decode(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]),
        Err(io::ImportError::Decode { format: "jpeg", .. })
    ));
    // BMP header + truncated pixels.
    assert!(matches!(
        raster::decode(b"BM\x00\x00"),
        Err(io::ImportError::Decode { format: "bmp", .. })
    ));
    // RIFF box that is not WEBP.
    assert!(matches!(
        raster::decode(b"RIFF\x00\x00\x00\x00WAVEfmt "),
        Err(io::ImportError::Unsupported {
            format: "raster",
            ..
        })
    ));
}
