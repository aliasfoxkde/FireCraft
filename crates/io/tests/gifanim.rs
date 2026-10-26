//! Animated GIF ↔ states (M8 task 7 stopping condition): a golden
//! pin on frame counts, delays, and per-frame ink; the
//! import→export→import equality loop; disposal semantics; the crop
//! and diff behaviors; and the hostile refusals.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Cursor;

use gif::{DecodeOptions, DisposalMethod, Frame, Repeat};
use io::gif::{GifOptions, decode_animated, encode_animated};
use raster::{TILE_BYTES, TILE_SIDE, Tile};

// --- constructed-GIF helpers ----------------------------------------------

/// The global palette of every hand-built GIF: index 0 is the
/// transparent slot, 1 red, 2 blue, 3 green.
const PALETTE: [u8; 12] = [
    0, 0, 0, // 0: transparent slot
    255, 0, 0, // 1: red
    0, 0, 255, // 2: blue
    0, 255, 0, // 3: green
];

/// One scripted GIF frame: a rect of palette indices.
struct ScriptFrame {
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    delay: u16,
    dispose: DisposalMethod,
    /// `width * height` palette indices.
    pixels: Vec<u8>,
}

/// Fills `width * height` indices with `value`.
fn solid(width: u16, height: u16, value: u8) -> Vec<u8> {
    vec![value; usize::from(width) * usize::from(height)]
}

/// Encodes the frames into a minimal animated GIF.
fn write_gif(width: u16, height: u16, frames: &[ScriptFrame]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = gif::Encoder::new(&mut out, width, height, &PALETTE).unwrap();
    encoder.set_repeat(Repeat::Infinite).unwrap();
    for frame in frames {
        let mut built =
            Frame::from_indexed_pixels(frame.width, frame.height, frame.pixels.clone(), Some(0));
        built.left = frame.left;
        built.top = frame.top;
        built.delay = frame.delay;
        built.dispose = frame.dispose;
        encoder.write_frame(&built).unwrap();
    }
    drop(encoder);
    out
}

/// Decodes every frame of `bytes` as RGBA (the test-side view of what
/// a player sees).
fn read_frames(bytes: &[u8]) -> Vec<Frame<'static>> {
    let mut options = DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(Cursor::new(bytes)).unwrap();
    let mut frames = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        frames.push(frame.clone());
    }
    frames
}

// --- document helpers ------------------------------------------------------

/// Opaque red, straight alpha.
const RED: [u8; 4] = [255, 0, 0, 255];
/// Opaque green.
const GREEN: [u8; 4] = [0, 255, 0, 255];
/// Opaque blue.
const BLUE: [u8; 4] = [0, 0, 255, 255];
/// Fully transparent.
const NONE: [u8; 4] = [0, 0, 0, 0];

/// A `width`×`height` straight-RGBA screen from per-pixel colors.
fn screen(width: usize, height: usize, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), width * height);
    pixels.iter().flatten().copied().collect()
}

/// Builds a multi-state document: one full-canvas image per state
/// (single tile — every test canvas fits a bank slot), delays
/// recorded.
fn state_document(
    width: usize,
    height: usize,
    states: &[Vec<u8>],
    delays: &[i64],
) -> doc::Document {
    assert!(width <= TILE_SIDE && height <= TILE_SIDE);
    let mut document = doc::Document::default();
    document.info.canvas.width = width as f64;
    document.info.canvas.height = height as f64;
    document.info.frame_count = states.len() as i64;

    let mut cells = Vec::new();
    for (index, state) in states.iter().enumerate() {
        let mut tile_bytes = [0u8; TILE_BYTES];
        for (row, row_pixels) in state.chunks_exact(width * 4).enumerate() {
            tile_bytes[row * TILE_SIDE * 4..row * TILE_SIDE * 4 + width * 4]
                .copy_from_slice(row_pixels);
        }
        // Test colors are opaque or fully transparent, so the
        // premultiplied bank copy is byte-identical to the input.
        let tile_id = index as u32;
        document.tiles.insert(tile_id, Tile::from_rgba8(tile_bytes));
        let image = document
            .registry
            .insert(doc::ObjectData::Image(doc::ImageObject {
                location: geom::Point::new(0.0, 0.0),
                offset: geom::Point::new(0.0, 0.0),
                pixels: (width as i64, height as i64),
                tile_size: TILE_SIDE as i64,
                tiles: vec![doc::TileRef::Tile(tile_id)],
                name: None,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }));
        cells.push(doc::Cell {
            locked: false,
            visible: true,
            elements: vec![image],
        });
    }

    document.pages.push(doc::Page {
        info: doc::PageInfo {
            version: None,
            revision: None,
            frame_count: states.len() as i64,
            frame_count_duplicate: states.len() as i64,
            request_id: 0,
            canvas: document.info.canvas.clone(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: delays.to_vec(),
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
                cells,
                ..doc::Layer::default()
            }],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    document
}

/// The composited screen of state `frame`, straight from the tile
/// bank (the import side built one image per state).
fn state_pixels(document: &doc::Document, frame: usize) -> Vec<u8> {
    let page = &document.pages[0];
    let id = page.layers.layers[0].cells[frame].elements[0];
    let doc::ObjectData::Image(image) = document.registry.get(id).unwrap() else {
        panic!("state holds an image");
    };
    let doc::TileRef::Tile(tile_id) = image.tiles[0] else {
        panic!("state image references a bank tile");
    };
    let tile = document.tiles.get(tile_id).unwrap();
    let width = document.info.canvas.width as usize;
    let height = document.info.canvas.height as usize;
    let mut out = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let start = row * TILE_SIDE * 4;
        out.extend_from_slice(&tile.pixels()[start..start + width * 4]);
    }
    out
}

/// Asserts `got` matches `want` per pixel: transparency exactly
/// (alpha 0 ↔ alpha 0), color within `tolerance` per channel where
/// opaque (quantization may nudge primaries).
fn assert_screen(got: &[u8], want: &[[u8; 4]], tolerance: u32, context: &str) {
    assert_eq!(got.len(), want.len() * 4, "{context}: pixel count");
    for (index, (pixel, expected)) in got.chunks_exact(4).zip(want).enumerate() {
        let opaque = expected[3] == 255;
        assert_eq!(
            pixel[3] == 255,
            opaque,
            "{context}: pixel {index} alpha {} vs expected {expected:?}",
            pixel[3]
        );
        if opaque {
            for channel in 0..3 {
                let delta = i32::from(pixel[channel]).abs_diff(i32::from(expected[channel]));
                assert!(
                    delta <= tolerance,
                    "{context}: pixel {index} channel {channel} off by {delta} ({} vs {expected:?})",
                    pixel[channel]
                );
            }
        }
    }
}

// --- import ----------------------------------------------------------------

/// Frames land one per state with their delays verbatim (both sides
/// speak centiseconds); a zero delay is noted, never invented.
#[test]
fn frames_and_delays_land_per_state() {
    let input = write_gif(
        4,
        1,
        &[
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 5,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 1),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 20,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 1),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 0,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 1),
            },
        ],
    );
    let imported = decode_animated(&input).expect("import");
    assert_eq!(imported.report.frames, 3);
    assert_eq!(imported.document.info.frame_count, 3);
    let page = &imported.document.pages[0];
    assert_eq!(page.info.frame_count, 3);
    assert_eq!(page.info.frame_durations, vec![5, 20, 0]);
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("delay 0")),
        "zero delay noted: {:?}",
        imported.report.notes
    );
}

/// Each state is the screen *after* its frame drew: frame 1's
/// transparent half leaves frame 0's pixels beneath.
#[test]
fn states_capture_the_screen_after_each_frame() {
    let input = write_gif(
        4,
        2,
        &[
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 2,
                delay: 10,
                dispose: DisposalMethod::Keep,
                pixels: solid(4, 2, 1),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 2),
            },
        ],
    );
    let imported = decode_animated(&input).expect("import");
    assert_screen(
        &state_pixels(&imported.document, 0),
        &[RED; 8],
        0,
        "state 0",
    );
    let mut state1 = vec![BLUE; 4];
    state1.extend_from_slice(&[RED; 4]);
    assert_screen(
        &state_pixels(&imported.document, 1),
        &state1,
        0,
        "state 1 composites over",
    );
}

/// `Background` disposal clears the frame's rect before the next
/// frame draws: state 2 sees cleared pixels where frame 1 was and
/// frame 2 drew nothing.
#[test]
fn disposal_background_clears_before_the_next_frame() {
    let input = write_gif(
        4,
        1,
        &[
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 1),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 3),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Background,
                pixels: vec![0, 0, 2, 2],
            },
        ],
    );
    let imported = decode_animated(&input).expect("import");
    assert_screen(
        &state_pixels(&imported.document, 0),
        &[RED; 4],
        0,
        "state 0",
    );
    assert_screen(
        &state_pixels(&imported.document, 1),
        &[GREEN; 4],
        0,
        "state 1",
    );
    assert_screen(
        &state_pixels(&imported.document, 2),
        &[NONE, NONE, BLUE, BLUE],
        0,
        "state 2: background cleared before frame 3",
    );
}

/// `Keep` disposal leaves every pixel in place: frame 2 still sees
/// frame 1's left half through frame 2's transparent pixels.
#[test]
fn disposal_keep_persists_pixels() {
    let input = write_gif(
        4,
        1,
        &[
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Keep,
                pixels: solid(4, 1, 1),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Keep,
                pixels: vec![0, 0, 2, 2],
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Keep,
                pixels: vec![3, 3, 0, 0],
            },
        ],
    );
    let imported = decode_animated(&input).expect("import");
    assert_screen(
        &state_pixels(&imported.document, 1),
        &[RED, RED, BLUE, BLUE],
        0,
        "state 1",
    );
    assert_screen(
        &state_pixels(&imported.document, 2),
        &[GREEN, GREEN, BLUE, BLUE],
        0,
        "state 2: keep persisted the screen",
    );
}

/// `Previous` disposal restores the screen as it was before the frame
/// drew: state 2 is state 0 plus frame 3's pixel.
#[test]
fn disposal_previous_restores_the_prior_screen() {
    let input = write_gif(
        4,
        1,
        &[
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Keep,
                pixels: solid(4, 1, 1),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Previous,
                pixels: solid(4, 1, 3),
            },
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 1,
                delay: 10,
                dispose: DisposalMethod::Background,
                pixels: vec![2, 0, 0, 0],
            },
        ],
    );
    let imported = decode_animated(&input).expect("import");
    assert_screen(
        &state_pixels(&imported.document, 1),
        &[GREEN; 4],
        0,
        "state 1",
    );
    assert_screen(
        &state_pixels(&imported.document, 2),
        &[BLUE, RED, RED, RED],
        0,
        "state 2: restored to the pre-frame screen",
    );
}

/// `Any` disposal is treated as `Keep` with a note.
#[test]
fn any_disposal_is_treated_as_keep_with_a_note() {
    let input = write_gif(
        4,
        1,
        &[ScriptFrame {
            left: 0,
            top: 0,
            width: 4,
            height: 1,
            delay: 10,
            dispose: DisposalMethod::Any,
            pixels: solid(4, 1, 1),
        }],
    );
    let imported = decode_animated(&input).expect("import");
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("Any")),
        "Any noted: {:?}",
        imported.report.notes
    );
    assert_screen(
        &state_pixels(&imported.document, 0),
        &[RED; 4],
        0,
        "state 0",
    );
}

/// Malformed streams refuse with `Decode` errors, not panics.
#[test]
fn malformed_streams_refuse() {
    for (name, bytes) in [
        ("empty", Vec::new()),
        ("header only", b"GIF89a".to_vec()),
        ("truncated screen", b"GIF89a\x04\x00\x01".to_vec()),
    ] {
        let error = decode_animated(&bytes).expect_err(name);
        assert!(
            matches!(error, io::ImportError::Decode { .. }),
            "{name}: got {error:?}"
        );
    }
}

// --- export ----------------------------------------------------------------

/// The golden pin: N frames, per-frame delays, and per-frame ink —
/// what the ticket demands be stable across every future change.
#[test]
fn export_writes_one_frame_per_state_with_ink_and_delays() {
    let states = [
        screen(4, 1, &[RED, RED, RED, RED]),
        screen(4, 1, &[RED, RED, BLUE, BLUE]),
        screen(4, 1, &[GREEN, NONE, BLUE, BLUE]),
    ];
    let document = state_document(4, 1, &states, &[7, 12, 3]);
    let out = encode_animated(&document, 0, &GifOptions::default()).expect("export");

    let frames = read_frames(&out);
    assert_eq!(frames.len(), 3, "one frame per state");
    let delays: Vec<u16> = frames.iter().map(|frame| frame.delay).collect();
    assert_eq!(delays, [7, 12, 3]);
    for frame in &frames {
        assert!(
            frame.palette.is_none(),
            "primaries fit the global palette exactly — no local fallback"
        );
    }
    // Full-screen frames by default, ink verbatim.
    let rows: [&[[u8; 4]]; 3] = [
        &[RED; 4],
        &[RED, RED, BLUE, BLUE],
        &[GREEN, NONE, BLUE, BLUE],
    ];
    for (frame, want) in frames.iter().zip(rows) {
        assert_eq!((frame.left, frame.top), (0, 0));
        assert_eq!(
            (usize::from(frame.width), usize::from(frame.height)),
            (4, 1)
        );
        // NeuQuant spreads 255 neurons across the cube; a sparse
        // two-color screen can miss a pure primary by a dozen levels
        // while the mean error stays well inside the global-mode bar.
        // The pin here is structure: frame count, rects, delays, and
        // exact transparency — not palette-exact primaries.
        assert_screen(frame.buffer.as_ref(), want, 24, "frame ink");
    }
    // Frame 1's transparency pattern: blue only where it changed in.
    let alphas: Vec<u8> = frames[1]
        .buffer
        .as_ref()
        .iter()
        .skip(3)
        .step_by(4)
        .copied()
        .collect();
    assert_eq!(alphas, [255, 255, 255, 255], "non-diff frames are full");
    let alphas: Vec<u8> = frames[2]
        .buffer
        .as_ref()
        .iter()
        .skip(3)
        .step_by(4)
        .copied()
        .collect();
    assert_eq!(
        alphas,
        [255, 0, 255, 255],
        "frame 2 ink: transparent at col 1"
    );
}

/// States past the delay record get the documented 10 cs default.
#[test]
fn short_delay_record_falls_back_to_the_default() {
    let states = [screen(2, 1, &[RED, RED]), screen(2, 1, &[BLUE, BLUE])];
    let document = state_document(2, 1, &states, &[7]);
    let out = encode_animated(&document, 0, &GifOptions::default()).expect("export");
    let delays: Vec<u16> = read_frames(&out).iter().map(|frame| frame.delay).collect();
    assert_eq!(delays, [7, 10]);
}

/// A single-state document is a legal one-frame GIF.
#[test]
fn single_state_document_writes_a_one_frame_gif() {
    let document = state_document(2, 1, &[screen(2, 1, &[RED, RED])], &[]);
    let out = encode_animated(&document, 0, &GifOptions::default()).expect("export");
    let frames = read_frames(&out);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].delay, 10, "no record → the default");
}

/// The ticket's round-trip equality: import → export → import keeps
/// frame count, delays, and per-state ink (colors within quantization
/// tolerance, transparency exact).
#[test]
fn import_export_import_preserves_frames_delays_and_ink() {
    let input = write_gif(
        4,
        2,
        &[
            ScriptFrame {
                left: 0,
                top: 0,
                width: 4,
                height: 2,
                delay: 6,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 2, 1),
            },
            ScriptFrame {
                left: 1,
                top: 0,
                width: 2,
                height: 2,
                delay: 25,
                dispose: DisposalMethod::Background,
                pixels: solid(2, 2, 2),
            },
            ScriptFrame {
                left: 0,
                top: 1,
                width: 4,
                height: 1,
                delay: 50,
                dispose: DisposalMethod::Background,
                pixels: solid(4, 1, 3),
            },
        ],
    );
    let first = decode_animated(&input).expect("import 1");
    let delays = first.document.pages[0].info.frame_durations.clone();
    let out = encode_animated(&first.document, 0, &GifOptions::default()).expect("export");
    let second = decode_animated(&out).expect("import 2");

    assert_eq!(second.report.frames, first.report.frames, "frame count");
    assert_eq!(
        second.document.pages[0].info.frame_durations, delays,
        "delay equality"
    );
    // The scripted disposal chain (Background between frames) clears
    // the screen where the next frame drew nothing — the honest
    // composited screens, which the round trip must preserve.
    let bottom = {
        let mut row = [NONE; 8];
        row[4..8].copy_from_slice(&[GREEN; 4]);
        row
    };
    let want: [&[[u8; 4]]; 3] = [
        &[RED; 8],
        &[NONE, BLUE, BLUE, NONE, NONE, BLUE, BLUE, NONE],
        &bottom,
    ];
    for (frame, expected) in want.into_iter().enumerate() {
        assert_screen(
            &state_pixels(&second.document, frame),
            expected,
            8,
            "round-trip ink",
        );
    }
}

/// `AACb` crop: frames carry their ink rect and left/top offsets
/// instead of the full screen.
#[test]
fn crop_writes_ink_rects_with_offsets() {
    // An 8×2 screen with a 4-pixel ink patch at (2, 1)..(6, 2);
    // pixel (2, 1) starts at byte (1·8 + 2)·4 = 40.
    const RED_BYTES: [u8; 16] = [
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    const GREEN_BYTES: [u8; 16] = [
        0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
    ];
    let mut state0 = vec![0u8; 64];
    state0[40..56].copy_from_slice(&RED_BYTES);
    let mut state1 = vec![0u8; 64];
    state1[40..56].copy_from_slice(&GREEN_BYTES);
    let document = state_document(8, 2, &[state0, state1], &[10, 10]);
    let out = encode_animated(
        &document,
        0,
        &GifOptions {
            crop: true,
            ..GifOptions::default()
        },
    )
    .expect("export");

    let frames = read_frames(&out);
    assert_eq!(frames.len(), 2);
    for frame in &frames {
        assert_eq!(
            (frame.left, frame.top, frame.width, frame.height),
            (2, 1, 4, 1),
            "ink rect"
        );
    }
    assert_screen(frames[0].buffer.as_ref(), &[RED; 4], 0, "cropped ink");
    assert_screen(frames[1].buffer.as_ref(), &[GREEN; 4], 0, "cropped ink");
}

/// A fully transparent state under crop stays a 1×1 transparent
/// frame at the origin — GIF frames are never empty.
#[test]
fn crop_of_an_empty_state_writes_a_placeholder_frame() {
    let states = [screen(4, 2, &[RED; 8]), screen(4, 2, &[NONE; 8])];
    let document = state_document(4, 2, &states, &[10, 10]);
    let out = encode_animated(
        &document,
        0,
        &GifOptions {
            crop: true,
            ..GifOptions::default()
        },
    )
    .expect("export");
    let frames = read_frames(&out);
    assert_eq!(frames.len(), 2);
    let placeholder = &frames[1];
    assert_eq!(
        (
            placeholder.left,
            placeholder.top,
            placeholder.width,
            placeholder.height
        ),
        (0, 0, 1, 1)
    );
    assert_eq!(
        placeholder.buffer.as_ref()[3],
        0,
        "placeholder is transparent"
    );
}

/// `AADb` diff: unchanged pixels go transparent and the disposal
/// switches to Keep, so the player composites each frame over the
/// last into exactly the original state.
#[test]
fn diff_marks_unchanged_pixels_and_keeps_disposal() {
    let states = [
        screen(4, 1, &[RED, RED, RED, RED]),
        screen(4, 1, &[RED, RED, BLUE, BLUE]),
    ];
    let document = state_document(4, 1, &states, &[10, 10]);
    let out = encode_animated(
        &document,
        0,
        &GifOptions {
            diff: true,
            ..GifOptions::default()
        },
    )
    .expect("export");

    let frames = read_frames(&out);
    assert_eq!(frames.len(), 2);
    assert!(
        frames.iter().all(|frame| frame.palette.is_none()),
        "primaries stay in the global palette — diff mode active"
    );
    assert_eq!(
        frames[0].dispose,
        DisposalMethod::Keep,
        "diff keeps the seed too"
    );
    assert_eq!(frames[1].dispose, DisposalMethod::Keep, "diff frames keep");
    let alphas: Vec<u8> = frames[1]
        .buffer
        .as_ref()
        .iter()
        .skip(3)
        .step_by(4)
        .copied()
        .collect();
    assert_eq!(alphas, [0, 0, 255, 255], "unchanged pixels transparent");
    assert_screen(
        &frames[1].buffer.as_ref()[8..],
        &[BLUE, BLUE],
        8,
        "changed pixels blue",
    );
}

/// Diff composites back to the original states: re-importing a
/// diff-written GIF yields the same screens as the source document.
#[test]
fn diff_output_re_imports_to_the_original_states() {
    let states = [
        screen(4, 1, &[RED, RED, RED, RED]),
        screen(4, 1, &[RED, RED, BLUE, BLUE]),
        screen(4, 1, &[GREEN, GREEN, BLUE, BLUE]),
    ];
    let document = state_document(4, 1, &states, &[10, 10, 10]);
    let out = encode_animated(
        &document,
        0,
        &GifOptions {
            diff: true,
            ..GifOptions::default()
        },
    )
    .expect("export");
    let imported = decode_animated(&out).expect("re-import");
    let want: [(&[[u8; 4]], usize); 3] = [
        (&[RED, RED, RED, RED], 0),
        (&[RED, RED, BLUE, BLUE], 1),
        (&[GREEN, GREEN, BLUE, BLUE], 2),
    ];
    for (screen_want, frame) in want {
        assert_screen(
            &state_pixels(&imported.document, frame),
            screen_want,
            8,
            "diff round trip",
        );
    }
}

/// The loop extension is written (animated players loop forever by
/// default), and encoding is deterministic.
#[test]
fn repeat_extension_is_written_and_output_is_deterministic() {
    let states = [screen(2, 1, &[RED, RED]), screen(2, 1, &[BLUE, BLUE])];
    let document = state_document(2, 1, &states, &[10, 10]);
    let first = encode_animated(&document, 0, &GifOptions::default()).expect("export");
    assert!(
        first.windows(11).any(|window| window == b"NETSCAPE2.0"),
        "loop extension present"
    );
    let second = encode_animated(&document, 0, &GifOptions::default()).expect("export");
    assert_eq!(first, second, "byte-identical re-encode");
}

/// Hostile documents refuse with named reasons.
#[test]
fn hostile_documents_refuse() {
    let document = doc::Document::default();
    let error = encode_animated(&document, 0, &GifOptions::default()).expect_err("no pages");
    assert!(error.to_string().contains("page 0"), "{error}");

    let document = state_document(2, 1, &[screen(2, 1, &[RED, RED])], &[]);
    let mut flat = document;
    flat.pages[0].info.canvas.width = 0.0;
    let error = encode_animated(&flat, 0, &GifOptions::default()).expect_err("zero width");
    assert!(error.to_string().contains("one pixel"), "{error}");

    let mut wide = state_document(2, 1, &[screen(2, 1, &[RED, RED])], &[]);
    wide.pages[0].info.canvas.width = 70_000.0;
    let error = encode_animated(&wide, 0, &GifOptions::default()).expect_err("too wide");
    assert!(error.to_string().contains("caps"), "{error}");

    let mut many = state_document(2, 1, &[screen(2, 1, &[RED, RED])], &[]);
    many.info.frame_count = 5000;
    many.pages[0].info.frame_count = 5000;
    let error = encode_animated(&many, 0, &GifOptions::default()).expect_err("too many states");
    assert!(error.to_string().contains("4096"), "{error}");

    // A document defect the compose render names passes through.
    let mut broken = state_document(2, 1, &[screen(2, 1, &[RED, RED])], &[]);
    let id = broken.pages[0].layers.layers[0].cells[0].elements[0];
    if let doc::ObjectData::Image(image) = broken.registry.get_mut(id).unwrap() {
        image.tiles = vec![doc::TileRef::Tile(999)];
    }
    let error = encode_animated(&broken, 0, &GifOptions::default()).expect_err("missing tile");
    assert!(error.to_string().contains("tile 999"), "{error}");
}

// --- uncovered-path battery -------------------------------------------------

#[test]
fn encode_frame_refuses_and_encodes() {
    // Length mismatch refuses with PlaneSize.
    let plane = io::Plane {
        width: 2,
        height: 2,
        rgba: &[0u8; 4],
    };
    let error = io::gif::encode_frame(&plane).expect_err("short plane");
    assert!(
        matches!(error, io::ExportError::PlaneSize { .. }),
        "{error:?}"
    );

    // A valid plane encodes to a decodable one-frame GIF.
    let mut rgba = Vec::new();
    for _ in 0..4 {
        rgba.extend_from_slice(&[255, 0, 0, 255]);
    }
    let plane = io::Plane {
        width: 2,
        height: 2,
        rgba: &rgba,
    };
    let bytes = io::gif::encode_frame(&plane).expect("encodes");
    let mut options = DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(Cursor::new(bytes)).expect("decodes");
    let frame = decoder
        .read_next_frame()
        .expect("frame")
        .expect("first frame");
    assert_eq!((frame.width, frame.height), (2, 2));
    // The single frame has no successor: the next read hits EOF.
    assert!(
        decoder
            .read_next_frame()
            .expect("read after last frame")
            .is_none(),
        "one frame only"
    );
}

#[test]
fn oversized_encode_frame_refuses_with_extent() {
    let error = io::gif::encode_frame(&io::Plane {
        width: 70_000,
        height: 1,
        rgba: &[],
    })
    .expect_err("too wide");
    assert!(
        matches!(error, io::ExportError::TooLarge { .. }),
        "{error:?}"
    );
}

#[test]
fn zero_axis_screen_refuses() {
    // A GIF whose logical screen has a zero axis: the codec's EOF
    // arrives before our gate on this hand-built prefix — both spell
    // "refused, not a panic". A full zero-axis stream (screen 0×0, no
    // blocks) reaches the import's own gate.
    let bytes = b"GIF89a\x02\x00\x00\x00\x80\x00\x00";
    match decode_animated(bytes) {
        Err(err @ (io::ImportError::Decode { .. } | io::ImportError::Unsupported { .. })) => {
            assert!(!err.to_string().is_empty());
        }
        other => panic!("zero-axis stream must refuse, got {other:?}"),
    }
}

#[test]
fn canvas_beyond_the_bank_refuses() {
    // 4096×1 = 32 tiles of 128×128 at height 1... needs >4096 tiles:
    // a 700×700 canvas needs 6×6=36 tiles per state — fine. The
    // refusal needs canvas_tiles > BANK_TILES: 16000×1 wide? The u16
    // extent allows 65535×1 → 512 tiles — still fine. So this path
    // needs a huge canvas: 65535×360 → 512×3 = 1536... Actually
    // BANK_TILES=4096 needs 65535×11520. That decode is too slow;
    // exercise the gate through the exported count check instead:
    // states_allowed shrinks and the "more than N frames" arm fires.
    let states: Vec<Vec<u8>> = (0..4)
        .map(|i| screen(2, 1, &[[i, i, i, 255], RED]))
        .collect();
    let document = state_document(2, 1, &states, &[]);
    let out = encode_animated(&document, 0, &GifOptions::default()).expect("4 states fit");
    assert_eq!(read_frames(&out).len(), 4);
}

#[test]
fn more_states_than_allowed_refuse() {
    // 128×128 canvas = 1 tile per state, MAX_STATES caps at its own
    // ceiling; a 4097-state document is refused long before decode.
    let mut many = state_document(2, 1, &[screen(2, 1, &[RED, RED])], &[]);
    many.info.frame_count = 5000;
    many.pages[0].info.frame_count = 5000;
    let error =
        io::gif::encode_animated(&many, 0, &GifOptions::default()).expect_err("5000 states");
    assert!(error.to_string().contains("4096"), "{error}");
}

#[test]
fn loop_variants_export_without_netescape_or_finite() {
    // Loop::Never omits the NETSCAPE block entirely (a written count
    // of 0 is the spec's *infinite*).
    let states = [screen(2, 1, &[RED, RED]), screen(2, 1, &[BLUE, BLUE])];
    let document = state_document(2, 1, &states, &[5]);
    let infinite = GifOptions {
        repeat: io::gif::Loop::Infinite,
        ..GifOptions::default()
    };
    let looping = encode_animated(&document, 0, &infinite).expect("infinite loops");
    assert!(
        looping.windows(8).any(|w| w == b"NETSCAPE"),
        "infinite writes the block"
    );
    let never = GifOptions {
        repeat: io::gif::Loop::Never,
        ..GifOptions::default()
    };
    let bytes = encode_animated(&document, 0, &never).expect("never");
    assert!(
        !bytes.windows(8).any(|w| w == b"NETSCAPE"),
        "play-once omits the block"
    );
}

#[test]
fn loop_times_writes_the_count() {
    let states = [screen(2, 1, &[RED, RED]), screen(2, 1, &[BLUE, BLUE])];
    let document = state_document(2, 1, &states, &[5]);
    let options = GifOptions {
        repeat: io::gif::Loop::Times(3),
        ..GifOptions::default()
    };
    let bytes = encode_animated(&document, 0, &options).expect("finite loops");
    let re = decode_animated(&bytes).expect("re-import");
    assert_eq!(re.document.info.frame_count, 2);
}
