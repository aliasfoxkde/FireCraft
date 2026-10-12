//! `EXPv` → codec option resolution (M8 task 8): the ABGR matte word,
//! the per-format option derivation, the GIF loop spellings (a written
//! NETSCAPE count of 0 *is* infinite, so play-never omits the
//! extension), Floyd–Steinberg dithering on the global palette, and
//! the loud refusal of interlaced PNG output.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Cursor;

use gif::DecodeOptions;
use io::expv::{StatedJpeg, gif_options, jpeg_options, matte_from_abgr, png_options};
use io::gif::{GifOptions, Loop, decode_animated, encode_animated};
use io::{Matte, png};

// --- ABGR matte ----------------------------------------------------------

#[test]
fn matte_from_abgr_reads_the_dictionary_byte_order() {
    // FCLi 0x00_20_40_80 is ABGR: A=0x00 B=0x20 G=0x40 R=0x80.
    assert_eq!(matte_from_abgr(0x00_20_40_80), Matte::Rgb(0x80, 0x40, 0x20));
    // Every opaque word becomes its triplet (canonicalization is the
    // encoders' job).
    assert_eq!(matte_from_abgr(0x00_FF_FF_FF), Matte::Rgb(255, 255, 255));
    assert_eq!(matte_from_abgr(0x00_00_00_00), Matte::Rgb(0, 0, 0));
}

// --- GIF options ---------------------------------------------------------

#[test]
fn gif_options_map_the_loop_spellings() {
    use doc::ExportOptions;
    let options = |frame_loop: Option<i64>| {
        gif_options(Some(&ExportOptions {
            frame_loop,
            ..ExportOptions::default()
        }))
        .repeat
    };
    assert_eq!(options(Some(0)), Loop::Infinite);
    assert_eq!(options(Some(-1)), Loop::Never);
    assert_eq!(options(Some(4_294_967_295)), Loop::Never);
    assert_eq!(options(Some(5)), Loop::Times(5));

    // Unset defaults to infinite.
    assert_eq!(options(None), Loop::Infinite);
    assert_eq!(gif_options(None).repeat, Loop::Infinite);

    // Absurd repeat counts clamp instead of panicking.
    assert_eq!(options(Some(i64::MAX)), Loop::Times(u16::MAX));
}

#[test]
fn gif_options_carry_crop_diff_and_dither() {
    let export = doc::ExportOptions {
        anim_crop: Some(true),
        anim_diff: Some(true),
        dither: Some(250), // out-of-range input clamps to 100
        ..doc::ExportOptions::default()
    };
    let options = gif_options(Some(&export));
    assert!(options.crop);
    assert!(options.diff);
    assert_eq!(options.dither, 100);

    let defaults = gif_options(None);
    assert!(!defaults.crop);
    assert!(!defaults.diff);
    assert_eq!(defaults.dither, 0);
}

// --- JPEG / PNG options --------------------------------------------------

#[test]
fn jpeg_options_stated_flags_win_over_expv() {
    let export = doc::ExportOptions {
        jpeg_quality: Some(20),
        jpeg_progressive: Some(true),
        matte_raw: Some(0x00_10_20_30),
        ..doc::ExportOptions::default()
    };

    let stated = StatedJpeg {
        quality: Some(88),
        progressive: false,
        matte: Matte::Black,
    };
    let options = jpeg_options(Some(&export), stated);
    assert_eq!(options.quality, 88, "explicit CLI quality wins");
    assert!(options.progressive, "EXPv progressive applies");
    assert_eq!(options.matte, Matte::Black, "explicit matte wins");
}

#[test]
fn jpeg_options_fall_back_to_expv_then_defaults() {
    let export = doc::ExportOptions {
        jpeg_quality: Some(72),
        matte_raw: Some(0x00_10_20_30),
        ..doc::ExportOptions::default()
    };
    let options = jpeg_options(Some(&export), StatedJpeg::default());
    assert_eq!(options.quality, 72);
    assert!(!options.progressive);
    assert_eq!(
        options.matte,
        Matte::Rgb(0x30, 0x20, 0x10),
        "EXPv matte applies while no explicit matte was stated"
    );

    // With no record at all: quality 90, white matte.
    let defaults = jpeg_options(None, StatedJpeg::default());
    assert_eq!(defaults.quality, 90);
    assert_eq!(defaults.matte, Matte::White);
}

#[test]
fn png_options_carry_interlace_and_encode_refuses_it() {
    let export = doc::ExportOptions {
        interlaced: Some(true),
        ..doc::ExportOptions::default()
    };
    let options = png_options(Some(&export));
    assert!(options.interlaced);

    let plane = io::Plane::new(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
    let error = png::encode_with(&plane, &options).expect_err("interlace is refused");
    assert!(
        error.to_string().contains("Adam7"),
        "refusal names the missing encoder: {error}"
    );

    assert!(!png_options(None).interlaced);
    png::encode_with(&plane, &png_options(None)).expect("plain PNG still encodes");
}

// --- dither through the real encoder -------------------------------------

#[test]
fn dither_changes_gradient_output_and_stays_decodable() {
    // More unique colours than the 256-slot global palette can hold:
    // the encoder must approximate, and error diffusion is what
    // spreads that approximation instead of banding it.
    let width = 32usize;
    let height = 10usize;
    let mut state = Vec::with_capacity(width * height * 4);
    for index in 0..width * height {
        let red = (index % 256) as u8;
        let green = (index / 256 * 200 + 40) as u8;
        state.extend_from_slice(&[red, green, 90, 255]);
    }

    let plain = GifOptions {
        repeat: Loop::Infinite,
        ..GifOptions::default()
    };
    let dithered = GifOptions {
        dither: 100,
        repeat: Loop::Infinite,
        ..GifOptions::default()
    };

    let build = |options: &GifOptions| {
        let mut document = super_state_document(&state, width, height);
        document.pages[0].export = None;
        encode_animated(&document, 0, options).expect("gradient encodes")
    };
    let plain_bytes = build(&plain);
    let dither_bytes = build(&dithered);

    // Both decode to the full frame set.
    for (label, bytes) in [
        ("plain", plain_bytes.clone()),
        ("dither", dither_bytes.clone()),
    ] {
        let frames = read_frame_count(&bytes);
        assert_eq!(frames, 1, "{label} keeps its single state");
    }
    // The dither flag actually reaches the quantizer: outputs differ.
    assert_ne!(plain_bytes, dither_bytes, "diffusion repaints the ramp");
}

#[test]
fn expv_dither_flag_reaches_the_encoder() {
    let width = 24usize;
    let height = 6usize;
    let mut state = Vec::with_capacity(width * height * 4);
    for _y in 0..height {
        for x in 0..width {
            let grey = 40 + (x * 120) / (width - 1).max(1);
            state.extend_from_slice(&[grey as u8, grey as u8, grey as u8, 255]);
        }
    }
    let mut document = super_state_document(&state, width, height);
    document.pages[0].export = Some(doc::ExportOptions {
        dither: Some(100),
        ..doc::ExportOptions::default()
    });

    let with_expv = encode_animated(
        &document,
        0,
        &gif_options(document.pages[0].export.as_ref()),
    )
    .expect("encodes");
    document.pages[0].export = None;
    let without = encode_animated(&document, 0, &GifOptions::default()).expect("encodes");
    assert_ne!(with_expv, without, "page EXPv dither steers the codec");
}

// --- helpers --------------------------------------------------------------

/// One-state document holding `rgba` as a single bank tile (the
/// gifanim harness shape, inlined to keep this file self-contained).
fn super_state_document(rgba: &[u8], width: usize, height: usize) -> doc::Document {
    use raster::{TILE_BYTES, TILE_SIDE, Tile};

    let mut document = doc::Document::default();
    document.info.canvas.width = width as f64;
    document.info.canvas.height = height as f64;
    document.info.frame_count = 1;

    let mut tile_bytes = [0u8; TILE_BYTES];
    for (row, row_pixels) in rgba.chunks_exact(width * 4).enumerate() {
        tile_bytes[row * TILE_SIDE * 4..row * TILE_SIDE * 4 + width * 4]
            .copy_from_slice(row_pixels);
    }
    document.tiles.insert(0, Tile::from_rgba8(tile_bytes));
    let image = document
        .registry
        .insert(doc::ObjectData::Image(doc::ImageObject {
            location: geom::Point::new(0.0, 0.0),
            offset: geom::Point::new(0.0, 0.0),
            pixels: (width as i64, height as i64),
            tile_size: TILE_SIDE as i64,
            tiles: vec![doc::TileRef::Tile(0)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
    document.pages.push(doc::Page {
        info: doc::PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: document.info.canvas.clone(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: doc::LayerList {
            layers: vec![doc::Layer {
                name: "Layer 1".to_owned(),
                display: true,
                cells: vec![doc::Cell {
                    locked: false,
                    visible: true,
                    elements: vec![image],
                }],
                ..doc::Layer::default()
            }],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    document
}

/// Counts decoded frames.
fn read_frame_count(bytes: &[u8]) -> usize {
    let mut options = DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(Cursor::new(bytes)).unwrap();
    let mut count = 0;
    while decoder.read_next_frame().unwrap().is_some() {
        count += 1;
    }
    count
}

#[test]
fn public_surface_is_importable() {
    // decode_animated is exercised by the gifanim suite; this pins the
    // public surface this file's sibling assertions lean on.
    let _: fn(&[u8]) -> Result<io::gif::ImportedGif, io::ImportError> = decode_animated;
}

// --- GIF loop spellings through real bytes --------------------------------

#[test]
fn loop_never_omits_netscape_and_times_writes_count() {
    // Two states so the animation path runs; colours off-palette are
    // irrelevant — the assertion is on the extension stream.
    let state = vec![8u8; 16 * 4];
    let document = super_state_document(&state, 4, 4);

    let infinite = encode_animated(
        &document,
        0,
        &GifOptions {
            repeat: Loop::Infinite,
            ..GifOptions::default()
        },
    )
    .expect("encodes");
    assert!(
        infinite.windows(11).any(|w| w == b"NETSCAPE2.0"),
        "infinite loop writes the NETSCAPE extension"
    );

    let never = encode_animated(
        &document,
        0,
        &GifOptions {
            repeat: Loop::Never,
            ..GifOptions::default()
        },
    )
    .expect("encodes");
    assert!(
        !never.windows(11).any(|w| w == b"NETSCAPE2.0"),
        "play-never omits the extension entirely"
    );

    let times = encode_animated(
        &document,
        0,
        &GifOptions {
            repeat: Loop::Times(7),
            ..GifOptions::default()
        },
    )
    .expect("encodes");
    assert!(
        times.windows(11).any(|w| w == b"NETSCAPE2.0"),
        "Times(n) writes the extension"
    );
    // NETSCAPE2.0 block: 0x21 0xFF 0x0B then the text, then 0x03 0x01
    // and the count little-endian.
    let at = times
        .windows(11)
        .position(|w| w == b"NETSCAPE2.0")
        .expect("checked above");
    let count_at = at + 11 + 2;
    assert_eq!(times[count_at], 7, "loop count 7 written");
    assert_eq!(times[count_at + 1], 0);
}
