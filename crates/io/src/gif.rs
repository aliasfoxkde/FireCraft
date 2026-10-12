//! Animated GIF ↔ document states (M8 task 7).
//!
//! Import walks the GIF89a frame loop with browser compositing
//! semantics — the observable contract for an animation format: each
//! frame draws over the screen (transparent pixels leave what is
//! beneath), state *i* captures the screen after frame *i*, and the
//! frame's disposal method decides what the next frame starts from.
//! Each state lands as one `IMGv` over its own tile-bank range in
//! cell *i* of a single layer; delays land in
//! `PageInfo::frame_durations` (both sides speak centiseconds — the
//! GIF delay field's unit *is* 10 ms).
//!
//! Export renders each state through `compose::render`, quantizes to
//! one global palette (`color_quant`, trained on every state at once
//! so colors stay stable across frames), and falls back to a
//! per-frame local palette when the global quantization misses the
//! house fidelity bar. The `AACb` (crop) and `AADb` (diff) `EXPv`
//! behaviors ride [`GifOptions`] until task 8 wires `EXPv` to them.

use std::collections::HashMap;
use std::io::Cursor;

use color_quant::NeuQuant;
use doc::{Cell, Document, Layer, LayerList, Page, PageInfo};
use gif::{ColorOutput, DecodeOptions, DisposalMethod, Frame, Repeat};
use raster::TILE_SIDE;

use crate::{ExportError, ImportError};

/// State ceiling: one `IMGv` per state, and the model's tile bank
/// holds at most 4096 tiles in total (`raster::attach_image`'s rule —
/// the budget below divides it across states).
const MAX_STATES: usize = 4096;
/// The tile bank is shared by every state's image; states × tiles
/// must fit it.
const BANK_TILES: usize = 4096;
/// Export delay when a document records none: 10 centiseconds (100
/// ms), the convention for unanimated frame data. The CS6 panel
/// default was not observable without the binary — revisit against
/// real-corpus `FNDi` evidence when the `EXPv` mapping lands (M8
/// task 8).
const DEFAULT_DELAY_CS: i64 = 10;
/// Mean absolute channel error above which a frame gets its own
/// local palette instead of the global quantization.
const LOCAL_PALETTE_ERROR: f64 = 8.0;

/// What an animated GIF import produced, plus the honest notes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GifImportReport {
    /// Frames decoded (and states built).
    pub frames: usize,
    /// Non-fatal divergences: what the file did that the model maps
    /// approximately.
    pub notes: Vec<String>,
}

/// An animated GIF decoded into a multi-state document.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedGif {
    /// One page; one layer; one `IMGv` per state in `cells[i]`.
    pub document: Document,
    /// Frame count and notes.
    pub report: GifImportReport,
}

/// Decodes an animated GIF into a multi-state document (one `IMGv`
/// per composited state, delays in `frame_durations`).
///
/// # Errors
/// [`ImportError`] for malformed streams, over-limit frame counts,
/// and tile budgets the bank cannot hold.
pub fn decode_animated(input: &[u8]) -> Result<ImportedGif, ImportError> {
    let mut options = DecodeOptions::new();
    options.set_color_output(ColorOutput::RGBA);
    let mut decoder =
        options
            .read_info(Cursor::new(input))
            .map_err(|error| ImportError::Decode {
                format: "gif",
                reason: error.to_string(),
            })?;
    let (width, height) = (decoder.width(), decoder.height());
    if width == 0 || height == 0 {
        return Err(ImportError::Unsupported {
            format: "gif",
            reason: "logical screen has a zero axis".to_owned(),
        });
    }

    // The bank holds every state's tiles; budget before decoding.
    let side = u16::try_from(TILE_SIDE).unwrap_or(u16::MAX);
    let canvas_tiles =
        usize::from(width.div_ceil(side)).max(1) * usize::from(height.div_ceil(side)).max(1);
    let states_allowed = (BANK_TILES / canvas_tiles).min(MAX_STATES);
    if states_allowed == 0 {
        return Err(ImportError::Unsupported {
            format: "gif",
            reason: format!("{width}x{height} fills the whole {BANK_TILES}-tile bank in one state"),
        });
    }

    let (canvas_w, canvas_h) = (usize::from(width), usize::from(height));
    let mut canvas = vec![0u8; canvas_w * canvas_h * 4];
    let mut states: Vec<Vec<u8>> = Vec::new();
    let mut delays: Vec<i64> = Vec::new();
    let mut report = GifImportReport::default();
    let mut any_disposal = false;

    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|error| ImportError::Decode {
            format: "gif",
            reason: error.to_string(),
        })?
    {
        if states.len() >= states_allowed {
            return Err(ImportError::Unsupported {
                format: "gif",
                reason: format!(
                    "more than {states_allowed} frames: {canvas_tiles} canvas tile(s) × the \
                     frame count exceeds the {BANK_TILES}-tile bank"
                ),
            });
        }
        let (left, top) = (usize::from(frame.left), usize::from(frame.top));
        let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
        if left + fw > canvas_w || top + fh > canvas_h {
            return Err(ImportError::Decode {
                format: "gif",
                reason: format!(
                    "frame {fw}x{fh} at ({left}, {top}) leaves the {canvas_w}x{canvas_h} screen"
                ),
            });
        }
        if frame.delay == 0 {
            report.notes.push(format!(
                "frame {} delay 0 — timing left to the player",
                states.len() + 1
            ));
        }
        if frame.dispose == DisposalMethod::Any {
            any_disposal = true;
        }
        // "Restore to previous" restores the screen as it was before
        // this frame drew, so the snapshot comes first.
        let snapshot = (frame.dispose == DisposalMethod::Previous).then(|| canvas.clone());
        composite_frame(
            &mut canvas,
            canvas_w,
            frame.buffer.as_ref(),
            fw,
            fh,
            left,
            top,
        );
        // State i = the screen after frame i drew.
        states.push(canvas.clone());
        delays.push(i64::from(frame.delay));
        match frame.dispose {
            DisposalMethod::Keep | DisposalMethod::Any => {}
            DisposalMethod::Background => {
                clear_rect(&mut canvas, canvas_w, left, top, fw, fh);
            }
            DisposalMethod::Previous => {
                canvas = snapshot.unwrap_or_default();
            }
        }
    }
    if states.is_empty() {
        return Err(ImportError::Decode {
            format: "gif",
            reason: "no frames".to_owned(),
        });
    }
    if any_disposal {
        report.notes.push("disposal Any treated as Keep".to_owned());
    }
    report.frames = states.len();

    let count = states.len();
    let mut document = Document::default();
    document.info.frame_count = count as i64;
    document.info.canvas.width = f64::from(width);
    document.info.canvas.height = f64::from(height);
    // One image per state, each over its own dense bank range, in one
    // layer's per-state cells.
    let mut cells = Vec::with_capacity(count);
    for state in &states {
        let id = crate::raster::attach_image(
            &mut document,
            u32::from(width),
            u32::from(height),
            state,
            None,
        )?;
        cells.push(Cell {
            locked: false,
            visible: true,
            elements: vec![id],
        });
    }
    let canvas = document.info.canvas.clone();
    document.pages.push(Page {
        info: PageInfo {
            version: document.info.version,
            revision: document.info.revision,
            frame_count: count as i64,
            frame_count_duplicate: count as i64,
            request_id: 0,
            canvas,
            onion_before: 0,
            onion_after: 0,
            frame_durations: delays,
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![Layer {
                name: "Layer 1".to_owned(),
                display: true,
                cells,
                ..Layer::default()
            }],
        },
        grid: None,
        guides: Default::default(),
        export: None,
    });
    Ok(ImportedGif { document, report })
}

/// Composites one frame's straight RGBA pixels over the screen
/// (source-over; fully transparent source pixels leave what is
/// beneath — the reason each state captures after drawing, not
/// replacing).
fn composite_frame(
    canvas: &mut [u8],
    canvas_w: usize,
    pixels: &[u8],
    fw: usize,
    fh: usize,
    left: usize,
    top: usize,
) {
    for row in 0..fh {
        let src = &pixels[row * fw * 4..(row + 1) * fw * 4];
        for col in 0..fw {
            let [r, g, b, a] = [
                src[col * 4],
                src[col * 4 + 1],
                src[col * 4 + 2],
                src[col * 4 + 3],
            ];
            if a == 0 {
                continue;
            }
            let at = ((top + row) * canvas_w + left + col) * 4;
            if a == 255 {
                canvas[at..at + 4].copy_from_slice(&[r, g, b, a]);
                continue;
            }
            let [dr, dg, db, da] = [
                u32::from(canvas[at]),
                u32::from(canvas[at + 1]),
                u32::from(canvas[at + 2]),
                u32::from(canvas[at + 3]),
            ];
            // `a` is nonzero here, so the output alpha cannot be 0
            // either — no checked division needed.
            let (s_a, s) = (u32::from(a), 255 - u32::from(a));
            let out_a = s_a + da * s / 255;
            let blend = |sr: u32, dst: u32| ((sr * s_a + dst * da * s / 255) / out_a) as u8;
            canvas[at] = blend(u32::from(r), dr);
            canvas[at + 1] = blend(u32::from(g), dg);
            canvas[at + 2] = blend(u32::from(b), db);
            canvas[at + 3] = out_a as u8;
        }
    }
}

/// Clears a rectangle of the screen to fully transparent.
fn clear_rect(canvas: &mut [u8], canvas_w: usize, left: usize, top: usize, w: usize, h: usize) {
    for row in top..top + h {
        for col in left..left + w {
            let at = (row * canvas_w + col) * 4;
            canvas[at..at + 4].copy_from_slice(&[0, 0, 0, 0]);
        }
    }
}

/// Animated-GIF export behaviors, matching the `EXPv` `AACb`/`AADb`/
/// `FLPi`/`PLDi` settings (wired to the document in M8 task 8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GifOptions {
    /// `AACb` — crop each frame to its ink bounding box (frame rect +
    /// left/top offsets instead of full-screen frames).
    pub crop: bool,
    /// `AADb` — write only pixels that differ from the previous state
    /// (unchanged pixels stay transparent and the disposal switches
    /// to Keep). Transparent-to-transparent changes are free; a state
    /// that *clears* a previously opaque pixel cannot be expressed —
    /// Keep means "leave what is there" — the same trade every
    /// diff-writing GIF exporter makes.
    pub diff: bool,
    /// `FLPi` — how the animation loops.
    pub repeat: Loop,
    /// `PLDi` — Floyd–Steinberg error-diffusion amount 0..=100
    /// (percent of the quantization error carried to neighbor pixels;
    /// 0 = plain nearest-color). Applies to the global-palette mode;
    /// local palettes quantize internally and are already
    /// full-fidelity.
    pub dither: u8,
}

/// `FLPi` — frame loop. `0` loops forever, `-1` (the dictionary's
/// `4294967295`) plays once with no loop extension at all (a NETSCAPE
/// count of `0` *means* infinite, so "never" must omit it), `n`
/// repeats `n` times.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Loop {
    /// Loop forever (the animated default; NETSCAPE count 0).
    #[default]
    Infinite,
    /// Play once, no NETSCAPE extension.
    Never,
    /// Repeat this many extra times (NETSCAPE count).
    Times(u16),
}

/// Encodes a page's states as an animated GIF: one compose render per
/// state, delays from `frame_durations`, and one of two palette
/// modes — a global palette trained on every state at once (which
/// enables `crop` and `diff`), or per-frame local palettes when the
/// global quantization misses the fidelity bar for any state (diff is
/// meaningless across varying palettes, so it is off there).
///
/// # Errors
/// [`ExportError`] for pages outside the document, canvases outside
/// the u16 extent, state counts above 4096, and document defects the
/// compose render names (dangling ids, missing tiles).
pub fn encode_animated(
    document: &Document,
    page_index: usize,
    options: &GifOptions,
) -> Result<Vec<u8>, ExportError> {
    let page = document
        .pages
        .get(page_index)
        .ok_or(ExportError::Unsupported {
            format: "gif",
            reason: format!("page {page_index} is outside the document"),
        })?;
    let width_f = page.info.canvas.width.round();
    let height_f = page.info.canvas.height.round();
    if width_f < 1.0 || height_f < 1.0 {
        return Err(ExportError::Unsupported {
            format: "gif",
            reason: "canvas rounds to fewer than one pixel on an axis".to_owned(),
        });
    }
    if width_f > f64::from(u16::MAX) || height_f > f64::from(u16::MAX) {
        return Err(ExportError::TooLarge {
            format: "gif",
            width: width_f as u32,
            height: height_f as u32,
            limit: u32::from(u16::MAX),
        });
    }
    let (width, height) = (width_f as u16, height_f as u16);
    let count = usize::try_from(page.info.frame_count.max(1)).unwrap_or(usize::MAX);
    if count > MAX_STATES {
        return Err(ExportError::Unsupported {
            format: "gif",
            reason: format!("{count} states exceeds the {MAX_STATES}-state ceiling"),
        });
    }

    // One straight-alpha render per state; the palette trains on all
    // of them at once.
    let mut states: Vec<Vec<u8>> = Vec::with_capacity(count);
    for frame in 0..count {
        let rendered = compose::render(document, page_index, frame).map_err(|error| {
            ExportError::Unsupported {
                format: "gif",
                reason: error.to_string(),
            }
        })?;
        states.push(compose::straight_alpha(&rendered.pixels));
    }

    let palette = global_palette(&states);
    let nearest = NearestPalette::new(&palette);
    let amount = f32::from(options.dither.min(100)) / 100.0;
    let indexed: Vec<Vec<u8>> = states
        .iter()
        .map(|state| quantize(state, &nearest, usize::from(width), amount))
        .collect();
    let global_fits = states
        .iter()
        .zip(&indexed)
        .all(|(state, indices)| palette_error(state, indices, &nearest) <= LOCAL_PALETTE_ERROR);

    let (canvas_w, canvas_h) = (usize::from(width), usize::from(height));
    let mut out = Vec::new();
    let mut encoder = gif::Encoder::new(&mut out, width, height, &palette).map_err(|error| {
        ExportError::Encode {
            format: "gif",
            reason: error.to_string(),
        }
    })?;
    // "Play once" omits the extension entirely — a written NETSCAPE
    // count of 0 is the spec's spelling of *infinite*.
    if options.repeat != Loop::Never {
        encoder
            .set_repeat(match options.repeat {
                Loop::Times(count) => Repeat::Finite(count),
                Loop::Infinite | Loop::Never => Repeat::Infinite,
            })
            .map_err(|error| ExportError::Encode {
                format: "gif",
                reason: error.to_string(),
            })?;
    }

    if global_fits {
        // Indexed state 0 seeds the diff walk; the diff space is the
        // full-canvas global indexing regardless of cropping.
        let mut previous: Vec<u8> = indexed[0].clone();
        for (frame, state_indices) in indexed.iter().enumerate() {
            let diffing = options.diff && frame > 0;
            let indices = if diffing {
                quantize_diff(
                    &states[frame],
                    &nearest,
                    &previous,
                    usize::from(width),
                    amount,
                )
            } else {
                state_indices.clone()
            };
            // Diff mode is Keep everywhere — including the seed
            // frame: a Background seed would clear the screen the
            // very next frame's unchanged pixels rely on.
            let disposal = if options.diff {
                DisposalMethod::Keep
            } else {
                DisposalMethod::Background
            };
            let mut exported = if options.crop {
                cropped_frame(&indices, canvas_w, canvas_h, Some(0))?
            } else {
                full_frame(&indices, width, height)
            };
            set_timing(&mut exported, delay_for(page, frame), disposal);
            encoder
                .write_frame(&exported)
                .map_err(|error| ExportError::Encode {
                    format: "gif",
                    reason: error.to_string(),
                })?;
            previous = state_indices.clone();
        }
    } else {
        // Local palettes, full fidelity per frame: `from_rgba_speed`
        // asserts on length mismatches, which cannot happen here (the
        // buffers are w*h*4 by construction) but is refused rather
        // than trusted.
        for (frame, state) in states.iter().enumerate() {
            if state.len() != usize::from(width) * usize::from(height) * 4 {
                return Err(ExportError::PlaneSize {
                    width: u32::from(width),
                    height: u32::from(height),
                    len: state.len(),
                });
            }
            let mut rgba = state.clone();
            let local = Frame::from_rgba_speed(width, height, &mut rgba, 10);
            let mut exported = if options.crop {
                let transparent = local.transparent;
                let mut cropped =
                    cropped_frame(local.buffer.as_ref(), canvas_w, canvas_h, transparent)?;
                cropped.palette = local.palette;
                cropped
            } else {
                local
            };
            set_timing(
                &mut exported,
                delay_for(page, frame),
                DisposalMethod::Background,
            );
            encoder
                .write_frame(&exported)
                .map_err(|error| ExportError::Encode {
                    format: "gif",
                    reason: error.to_string(),
                })?;
        }
    }
    // The encoder borrows `out`; drop it so the bytes can move.
    drop(encoder);
    Ok(out)
}

/// Applies delay and disposal to a built frame.
fn set_timing(frame: &mut Frame<'static>, delay: u16, dispose: DisposalMethod) {
    frame.delay = delay;
    frame.dispose = dispose;
}

/// The delay for `frame` in centiseconds: the document's recorded
/// value, or the default when states outrun the record.
fn delay_for(page: &Page, frame: usize) -> u16 {
    u16::try_from(
        page.info
            .frame_durations
            .get(frame)
            .copied()
            .unwrap_or(DEFAULT_DELAY_CS)
            .clamp(0, i64::from(u16::MAX)),
    )
    .unwrap_or(0)
}

/// Trains the global palette on every state at once (opaque pixels
/// only — GIF transparency is binary and slot 0 stays reserved for
/// it), returning RGB triplets whose index 0 is the transparent slot.
fn global_palette(states: &[Vec<u8>]) -> Vec<u8> {
    let mut training = Vec::new();
    for state in states {
        for pixel in state.chunks_exact(4) {
            if pixel[3] >= 128 {
                training.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
    }
    let mut palette = vec![0u8; 3]; // the reserved transparent slot
    if training.is_empty() {
        // An all-transparent animation: one flat entry keeps the
        // table legal without pretending content exists.
        palette.extend_from_slice(&[255, 255, 255]);
        return palette;
    }
    // Sample factor 1: the full training set, deterministic.
    let quant = NeuQuant::new(1, 255, &training);
    palette.extend_from_slice(&quant.color_map_rgb());
    palette
}

/// Maps one state's straight RGBA onto the global palette (slot 0 is
/// transparency, never matched). With `amount` above zero the
/// quantization error diffuses Floyd–Steinberg style, scaled by the
/// amount — at `amount = 1.0` the classic weights (7/16, 3/16, 5/16,
/// 1/16), at 0 plain nearest-color. Transparent pixels (the reserved
/// slot) neither receive nor pass on error.
fn quantize(state: &[u8], nearest: &NearestPalette, width: usize, amount: f32) -> Vec<u8> {
    let mut indexed = Vec::with_capacity(state.len() / 4);
    if amount <= 0.0 {
        let mut memo: HashMap<[u8; 4], u8> = HashMap::new();
        for pixel in state.chunks_exact(4) {
            if pixel[3] < 128 {
                indexed.push(0);
                continue;
            }
            let key = [pixel[0], pixel[1], pixel[2], 255];
            indexed.push(*memo.entry(key).or_insert_with(|| nearest.index_of(pixel)));
        }
        return indexed;
    }

    // Running per-channel error, one slot per pixel (row-major; rows
    // below accumulate into their own slots).
    let height = state.len() / 4 / width.max(1);
    let mut error = vec![0.0f32; state.len() / 4 * 3];
    for (at, pixel) in state.chunks_exact(4).enumerate() {
        if pixel[3] < 128 {
            indexed.push(0);
            continue;
        }
        let base = at * 3;
        let want = [
            (f32::from(pixel[0]) + error[base]).clamp(0.0, 255.0),
            (f32::from(pixel[1]) + error[base + 1]).clamp(0.0, 255.0),
            (f32::from(pixel[2]) + error[base + 2]).clamp(0.0, 255.0),
        ];
        let want_bytes = [
            want[0].round() as u8,
            want[1].round() as u8,
            want[2].round() as u8,
        ];
        let index = nearest.index_of(&[want_bytes[0], want_bytes[1], want_bytes[2], 255]);
        indexed.push(index);
        let got = nearest.color_of(index);
        let carry = [
            (want[0] - f32::from(got[0])) * amount,
            (want[1] - f32::from(got[1])) * amount,
            (want[2] - f32::from(got[2])) * amount,
        ];
        // 7/16 right, 3/16 below-left, 5/16 below, 1/16 below-right.
        let (x, y) = (at % width, at / width);
        let mut give = |target: usize, share: f32| {
            if target < error.len() {
                for channel in 0..3 {
                    error[target * 3 + channel] += carry[channel] * share;
                }
            }
        };
        if x + 1 < width {
            give(at + 1, 7.0 / 16.0);
        }
        if y + 1 < height {
            if x > 0 {
                give(at + width - 1, 3.0 / 16.0);
            }
            give(at + width, 5.0 / 16.0);
            if x + 1 < width {
                give(at + width + 1, 1.0 / 16.0);
            }
        }
    }
    indexed
}

/// Maps onto the global palette, sending pixels whose index equals
/// the previous frame's to transparent slot 0 — an indexed-space
/// delta.
fn quantize_diff(
    state: &[u8],
    nearest: &NearestPalette,
    previous: &[u8],
    width: usize,
    amount: f32,
) -> Vec<u8> {
    let mut indexed = quantize(state, nearest, width, amount);
    for (index, at) in indexed.iter_mut().enumerate() {
        if previous[index] != 0 && previous[index] == *at {
            *at = 0;
        }
    }
    indexed
}

/// Nearest-color lookup against the fixed global palette with a
/// per-call memo (documents have few distinct colors; the scan is
/// linear). Slot 0 is the transparent slot, never matched.
struct NearestPalette {
    entries: Vec<[u8; 3]>,
}

impl NearestPalette {
    fn new(palette: &[u8]) -> Self {
        Self {
            entries: palette
                .chunks_exact(3)
                .map(|rgb| [rgb[0], rgb[1], rgb[2]])
                .collect(),
        }
    }

    fn index_of(&self, pixel: &[u8]) -> u8 {
        let mut best = 1usize;
        let mut best_error = u32::MAX;
        for (index, entry) in self.entries.iter().enumerate().skip(1) {
            let error = distance(pixel, entry);
            if error < best_error {
                best_error = error;
                best = index;
            }
        }
        u8::try_from(best).unwrap_or(u8::MAX)
    }

    /// The palette color behind `index` (black when out of range —
    /// only reachable through a corrupt index, which the diffusers
    /// never produce).
    fn color_of(&self, index: u8) -> [u8; 3] {
        self.entries
            .get(usize::from(index))
            .copied()
            .unwrap_or([0, 0, 0])
    }
}

/// Squared RGB distance — ordering is all the nearest scan needs.
fn distance(pixel: &[u8], entry: &[u8; 3]) -> u32 {
    let channel = |a: u8, b: u8| {
        let d = i32::from(a) - i32::from(b);
        u32::try_from(d * d).unwrap_or(u32::MAX)
    };
    channel(pixel[0], entry[0]) + channel(pixel[1], entry[1]) + channel(pixel[2], entry[2])
}

/// Mean absolute RGB error per channel between a state and its
/// indexed form — the fidelity bar the global palette must clear.
fn palette_error(state: &[u8], indexed: &[u8], nearest: &NearestPalette) -> f64 {
    let mut total = 0u64;
    let mut count = 0u64;
    for (index, pixel) in state.chunks_exact(4).enumerate() {
        if pixel[3] < 128 {
            continue;
        }
        let Some(entry) = nearest.entries.get(usize::from(indexed[index])) else {
            continue;
        };
        for channel in 0..3 {
            total += u64::from(pixel[channel].abs_diff(entry[channel]));
        }
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total as f64 / (count * 3) as f64
    }
}

/// Wraps a full-canvas indexed frame.
fn full_frame(indices: &[u8], width: u16, height: u16) -> Frame<'static> {
    build_indexed_frame(indices, width, height, 0, 0, Some(0))
}

/// Builds an indexed frame; every caller slices its indices out of a
/// canvas buffer, so the length contract `w*h` holds by construction.
fn build_indexed_frame(
    indices: &[u8],
    width: u16,
    height: u16,
    left: u16,
    top: u16,
    transparent: Option<u8>,
) -> Frame<'static> {
    // Callers slice these out of a canvas buffer, so the length is
    // the canvas's; the assert path below is unreachable by
    // construction and kept as the constructor's own contract.
    let mut frame = Frame::from_indexed_pixels(width, height, indices.to_vec(), transparent);
    frame.left = left;
    frame.top = top;
    frame
}

/// Crops an indexed frame to its nonzero-pixel bounding box. A fully
/// transparent frame stays a 1×1 transparent frame at the origin —
/// GIF frames are never empty.
fn cropped_frame(
    indices: &[u8],
    canvas_w: usize,
    canvas_h: usize,
    transparent: Option<u8>,
) -> Result<Frame<'static>, ExportError> {
    let ink = |value: u8| Some(value) != transparent;
    let mut min_x = canvas_w;
    let mut min_y = canvas_h;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    for (index, value) in indices.iter().enumerate() {
        if !ink(*value) {
            continue;
        }
        let (x, y) = (index % canvas_w, index / canvas_w);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    if min_x > max_x || min_y > max_y {
        let mut frame =
            Frame::from_indexed_pixels(1, 1, vec![transparent.unwrap_or(0)], transparent);
        frame.left = 0;
        frame.top = 0;
        return Ok(frame);
    }
    let fw = max_x - min_x + 1;
    let fh = max_y - min_y + 1;
    let mut buffer = Vec::with_capacity(fw * fh);
    for row in min_y..=max_y {
        let start = row * canvas_w + min_x;
        buffer.extend_from_slice(&indices[start..start + fw]);
    }
    Ok(build_indexed_frame(
        &buffer,
        u16::try_from(fw).unwrap_or(u16::MAX),
        u16::try_from(fh).unwrap_or(u16::MAX),
        u16::try_from(min_x).unwrap_or(0),
        u16::try_from(min_y).unwrap_or(0),
        transparent,
    ))
}
