//! Onion skinning compositor (CS6 phase 20.3).
//!
//! CS6's onion skinning draws the states adjacent to the current one as
//! translucent ghosts *behind* the current state, so the animator can
//! see where the motion comes from and where it goes. The shell owns
//! the settings (`PageInfo::onion_before` / `onion_after`, the `OSBi` /
//! `OSAi` nodes); this module owns the pixels.
//!
//! The composite order is the CS6 order: the canvas matte first, then
//! the ghosts, then the current state at full opacity. Ghosts are drawn
//! from the *content* of their state — without the matte — so a ghost
//! never paints the canvas colour over its neighbours.
//!
//! The ghost alpha is a flat 50%. CS6 exposes no per-ghost opacity
//! control, and a flat factor keeps the compositor deterministic: the
//! same document and the same settings always produce the same bytes.

use crate::cache::EffectCache;
use crate::render::{ComposeError, Render, render_scaled_inner};
use doc::Document;

/// The opacity factor every ghost is drawn at, as a numerator over 256.
const GHOST_ALPHA_NUM: u32 = 128;

/// The ghost frames for a current state.
///
/// `before` ghosts precede the current state, `after` ghosts follow it.
/// The two counts are independent — CS6's onion settings name a number
/// of states on each side of the current one, and a page may show two
/// states of history and none of the future.
///
/// The list is ordered by increasing distance from the current state, so
/// the nearest ghost composites last and sits on top of the farther
/// ones. Frames outside the page's frame count are dropped: a page
/// with five states cannot ghost a sixth.
///
/// The current state is never in the list — it is drawn opaque.
#[must_use]
pub fn ghost_frames(current: usize, before: usize, after: usize, total: usize) -> Vec<usize> {
    let mut ghosts = Vec::new();
    for distance in 1..=before.max(after) {
        if distance <= before
            && let Some(previous) = current.checked_sub(distance)
            && previous < total
        {
            ghosts.push(previous);
        }
        if distance <= after {
            let next = current + distance;
            if next < total {
                ghosts.push(next);
            }
        }
    }
    ghosts
}

/// Composites one page's onion-skinned view at `scale`× resolution.
///
/// Reads the onion settings from the page itself (`OSBi` / `OSAi`), so
/// the shell only has to set them — the compositor always agrees with
/// the document. The page's own frame count bounds the ghost set; a
/// page shows at least one state, the same rule the GIF exporter uses.
pub fn render_onion(
    document: &Document,
    page: usize,
    frame: usize,
    scale: f64,
) -> Result<Render, ComposeError> {
    let page_info = document
        .pages
        .get(page)
        .ok_or(ComposeError::PageOutOfBounds { page })?;
    let before = page_info.info.onion_before.max(0) as usize;
    let after = page_info.info.onion_after.max(0) as usize;
    let total = page_info.info.frame_count.max(1) as usize;
    render_onion_with(
        document,
        page,
        frame,
        scale,
        &ghost_frames(frame, before, after, total),
    )
}

/// [`render_onion`] over an explicit ghost list — the seam a host uses
/// when it wants to override the page's own settings.
pub fn render_onion_with(
    document: &Document,
    page: usize,
    frame: usize,
    scale: f64,
    ghosts: &[usize],
) -> Result<Render, ComposeError> {
    let mut cache = EffectCache::new();
    // The current state, content only: the matte is painted separately so
    // the ghosts can sit between it and the current state.
    let current = render_scaled_inner(document, page, frame, scale, &mut cache, false)?;
    let (width, height) = (current.width, current.height);
    let mut out = Render {
        width,
        height,
        pixels: vec![0; current.pixels.len()],
    };
    // The canvas matte, opaque, exactly as the plain render paints it.
    let page_info = document
        .pages
        .get(page)
        .ok_or(ComposeError::PageOutOfBounds { page })?;
    let raw = page_info.info.canvas.background_raw as u32;
    fill(
        &mut out.pixels,
        [
            (raw & 0xFF) as u8,
            ((raw >> 8) & 0xFF) as u8,
            ((raw >> 16) & 0xFF) as u8,
            255_u8,
        ],
    );
    // Ghosts, farthest first so the nearest sits on top.
    for ghost in ghosts.iter().rev() {
        let ghost_render = render_scaled_inner(document, page, *ghost, scale, &mut cache, false)?;
        over(&mut out.pixels, &ghost_render.pixels, GHOST_ALPHA_NUM);
    }
    // The current state on top of everything, at its own alpha.
    over(&mut out.pixels, &current.pixels, 256);
    Ok(out)
}

/// Fills every pixel with an opaque colour.
fn fill(pixels: &mut [u8], rgba: [u8; 4]) {
    for chunk in pixels.as_chunks_mut::<4>().0.iter_mut() {
        *chunk = rgba;
    }
}

/// Composites premultiplied `src` over `dst` at `alpha` (a numerator
/// over 256).
///
/// Both buffers are premultiplied sRGB RGBA8, so the standard `over`
/// operator applies directly: the source's colour and alpha are added,
/// and the destination is scaled down by the source's alpha. The alpha
/// factor scales the source's premultiplied colour as well, so a
/// half-opacity ghost keeps its own coverage.
///
/// Both buffers are a whole number of RGBA8 pixels — a `Render`'s
/// pixel vector always is — which is what the four-channel chunking
/// below relies on.
fn over(dst: &mut [u8], src: &[u8], alpha: u32) {
    let (dst_pixels, _) = dst.as_chunks_mut::<4>();
    let (src_pixels, _) = src.as_chunks::<4>();
    for (d, s) in dst_pixels.iter_mut().zip(src_pixels.iter()) {
        let sa = u32::from(s[3]) * alpha / 256;
        let inv = 255 - sa;
        let sr = u32::from(s[0]) * alpha / 256;
        let sg = u32::from(s[1]) * alpha / 256;
        let sb = u32::from(s[2]) * alpha / 256;
        d[0] = (sr + u32::from(d[0]) * inv / 255).min(255) as u8;
        d[1] = (sg + u32::from(d[1]) * inv / 255).min(255) as u8;
        d[2] = (sb + u32::from(d[2]) * inv / 255).min(255) as u8;
        d[3] = (sa + u32::from(d[3]) * inv / 255).min(255) as u8;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::{ghost_frames, render_onion, render_onion_with};
    use crate::Render;
    use crate::render::render_scaled;
    use doc::{
        Canvas, Cell, Contour, Document, Layer, LayerList, ObjectData, ObjectId, Page, PageInfo,
        PaintStyle, PathObject, PathPoint, Registry,
    };
    use geom::Point;

    /// A page of `count` states, each painting a solid rectangle of its
    /// own colour over one column band of an 8×8 canvas. The band
    /// layout makes every state individually readable from a single
    /// pixel, so a test can name the state it sees.
    fn document_with_states(count: usize) -> Document {
        let mut document = Document::default();
        let mut registry = Registry::new();
        let band = 8.0 / count as f64;
        let mut cells = Vec::new();
        for index in 0..count {
            let x = band * index as f64;
            let colour = (40 + index * 60) as u8;
            let id = registry.insert(ObjectData::Path(PathObject {
                contours: vec![rect_contour(x, 0.0, band, 8.0)],
                primitive: None,
                even_odd: false,
                paint: Some(PaintStyle {
                    stroke_raw: 0,
                    fill_raw: abgr(colour, colour, colour),
                    fill: None,
                }),
                fill_start: None,
                fill_end: None,
                fill_third: None,
                brush_placement: None,
                fill_on_top: None,
                locked: false,
                visible: true,
                opacity: 1000,
                blend: 0,
                name: None,
                effects: Vec::new(),
            }));
            cells.push(Cell {
                locked: false,
                visible: true,
                elements: vec![id],
            });
        }
        document.registry = registry;
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: count as i64,
                frame_count_duplicate: count as i64,
                request_id: 0,
                canvas: Canvas {
                    x: 0.0,
                    y: 0.0,
                    width: 8.0,
                    height: 8.0,
                    resolution: 72.0,
                    resolution_unit: 0,
                    background_raw: abgr(255, 255, 255),
                },
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
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    is_background: false,
                    has_url: false,
                    is_special: false,
                    frame_changing: true,
                    display: true,
                    nesting_level: None,
                    cells,
                    visibility_per_frame: None,
                    shared_link: None,
                }],
            },
            grid: None,
            guides: doc::Guides::default(),
            export: None,
        });
        document
    }

    /// The ABGR colour int the model carries (`FCLi` layout).
    fn abgr(r: u8, g: u8, b: u8) -> i64 {
        crate::convert::abgr_pack(r, g, b, 255)
    }

    fn rect_contour(x: f64, y: f64, width: f64, height: f64) -> Contour {
        let corners = [
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
        ];
        Contour {
            closed: true,
            point_count: 4,
            points: corners
                .iter()
                .map(|(px, py)| PathPoint {
                    anchor: Point::new(*px, *py),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                })
                .collect(),
        }
    }

    /// The pixel at `(x, y)` of a render.
    fn pixel(render: &Render, x: usize, y: usize) -> [u8; 4] {
        let index = (y * usize::from(render.width) + x) * 4;
        [
            render.pixels[index],
            render.pixels[index + 1],
            render.pixels[index + 2],
            render.pixels[index + 3],
        ]
    }

    #[test]
    fn the_ghost_set_is_the_neighbouring_states() {
        // Symmetric: one state on each side of the current one.
        assert_eq!(ghost_frames(2, 1, 1, 5), vec![1, 3]);
        assert_eq!(ghost_frames(2, 2, 2, 5), vec![1, 3, 0, 4]);
        // Asymmetric: two states of history, one of the future.
        assert_eq!(ghost_frames(2, 2, 1, 5), vec![1, 3, 0]);
        // The current state sits at the end of the page: only history
        // is left to ghost.
        assert_eq!(ghost_frames(4, 2, 2, 5), vec![3, 2]);
        // The current state sits at the start of the page: only the
        // future is left to ghost.
        assert_eq!(ghost_frames(0, 2, 1, 5), vec![1]);
    }

    #[test]
    fn the_before_and_after_counts_are_independent() {
        // Two states of history and none of the future.
        assert_eq!(ghost_frames(3, 2, 0, 5), vec![2, 1]);
        // None of the history and two of the future.
        assert_eq!(ghost_frames(1, 0, 2, 5), vec![2, 3]);
        // A wide history does not widen the future.
        assert_eq!(ghost_frames(3, 3, 1, 5), vec![2, 4, 1, 0]);
    }

    #[test]
    fn a_ghost_never_reaches_past_the_page() {
        assert_eq!(ghost_frames(0, 5, 5, 2), vec![1]);
        assert_eq!(ghost_frames(1, 3, 3, 2), vec![0]);
    }

    #[test]
    fn the_current_state_is_never_a_ghost() {
        assert!(!ghost_frames(1, 3, 3, 4).contains(&1));
        assert!(!ghost_frames(0, 3, 3, 4).contains(&0));
        assert!(!ghost_frames(3, 3, 3, 4).contains(&3));
    }

    #[test]
    fn the_ghost_set_is_empty_without_onion_skinning() {
        assert!(ghost_frames(1, 0, 0, 4).is_empty());
    }

    #[test]
    fn the_ghosts_are_ordered_by_distance_from_the_current_state() {
        // Nearest first: the compositor walks the list backwards so the
        // nearest ghost lands on top of the farther ones.
        let ghosts = ghost_frames(3, 3, 3, 7);
        assert_eq!(ghosts, vec![2, 4, 1, 5, 0, 6]);
        let distances: Vec<usize> = ghosts.iter().map(|frame| frame.abs_diff(3)).collect();
        assert!(
            distances.windows(2).all(|pair| pair[0] <= pair[1]),
            "the list is ordered by increasing distance: {distances:?}"
        );
    }

    #[test]
    fn onion_skinning_renders_the_expected_ghost_set() {
        // Four states, each painting one column band. The current state
        // is 2 (blue-ish, band 2); the ghosts are 1 and 3.
        let document = document_with_states(4);
        let onion = render_onion_with(&document, 0, 2, 1.0, &[1, 3]).expect("onion render");
        // Band 0 carries no ghost: the matte shows.
        assert_eq!(pixel(&onion, 0, 0), [255, 255, 255, 255]);
        // Band 1 carries the state-1 ghost at half alpha over white:
        // state 1's colour is 40 + 60 = 100, so 100/2 + 255/2 = 178.
        assert_eq!(pixel(&onion, 2, 0), [178, 178, 178, 255]);
        // Band 2 is the current state, opaque.
        assert_eq!(pixel(&onion, 4, 0), [160, 160, 160, 255]);
        // Band 3 carries the state-3 ghost at half alpha over white:
        // state 3's colour is 40 + 180 = 220, so 220/2 + 255/2 = 238.
        assert_eq!(pixel(&onion, 6, 0), [238, 238, 238, 255]);
    }

    #[test]
    fn the_current_state_is_drawn_over_the_ghosts() {
        let document = document_with_states(2);
        // Current state 1 paints band 1; the ghost (state 0) paints band
        // 0 — so the current state's band is untouched by the ghost.
        let onion = render_onion_with(&document, 0, 1, 1.0, &[0]).expect("onion render");
        assert_eq!(pixel(&onion, 4, 0), [100, 100, 100, 255]);
        assert_eq!(pixel(&onion, 0, 0), [148, 148, 148, 255]);
    }

    #[test]
    fn the_page_settings_drive_the_ghost_set() {
        let mut document = document_with_states(4);
        document.pages[0].info.onion_before = 1;
        document.pages[0].info.onion_after = 1;
        let onion = render_onion(&document, 0, 2, 1.0).expect("onion render");
        // Same expectations as the explicit ghost list.
        assert_eq!(pixel(&onion, 0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel(&onion, 2, 0), [178, 178, 178, 255]);
        assert_eq!(pixel(&onion, 4, 0), [160, 160, 160, 255]);
        assert_eq!(pixel(&onion, 6, 0), [238, 238, 238, 255]);
    }

    #[test]
    fn without_onion_skinning_the_render_matches_the_plain_one() {
        let document = document_with_states(4);
        let plain = render_scaled(&document, 0, 2, 1.0).expect("plain render");
        let onion = render_onion(&document, 0, 2, 1.0).expect("onion render");
        assert_eq!(onion.width, plain.width);
        assert_eq!(onion.height, plain.height);
        // Band 0 is matte in both; band 2 is the current state in both.
        assert_eq!(pixel(&onion, 0, 0), pixel(&plain, 0, 0));
        assert_eq!(pixel(&onion, 4, 0), pixel(&plain, 4, 0));
    }

    #[test]
    fn a_dangling_element_is_refused() {
        let mut document = document_with_states(1);
        document.pages[0].layers.layers[0].cells[0].elements = vec![ObjectId::from_parts(9, 0)];
        let result = render_onion_with(&document, 0, 0, 1.0, &[]);
        assert!(result.is_err(), "a dangling element must not render");
    }
}
