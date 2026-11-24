//! Mask rendering (task 143, decision 0092): a mask group restricts its
//! content by the mask's coverage, a stack of levels folds the
//! coverages with the level's combine mode, and a disabled mask group
//! draws its content unmasked.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::render;
use doc::{
    Document, GroupObject, Layer, LayerList, MaskCombineMode, ObjectData, ObjectId, Page, PageInfo,
    Registry, TileRef,
};
use geom::Point;
use raster::{TILE_BYTES, TILE_SIDE, Tile};

/// The canvas matte (`BGCi` default white, painted opaque).
const BACKGROUND: [u8; 4] = [255, 255, 255, 255];
/// Opaque red.
const RED: [u8; 4] = [255, 0, 0, 255];

// --- document builders ---------------------------------------------------

fn page(width: f64, height: f64, layers: Vec<Layer>) -> Page {
    Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 2,
            frame_count_duplicate: 2,
            request_id: 0,
            canvas: doc::Canvas {
                x: 0.0,
                y: 0.0,
                width,
                height,
                resolution: 72.0,
                resolution_unit: 0,
                background_raw: 0xFF_FF_FF,
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
        layers: LayerList { layers },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    }
}

fn layer(cells: Vec<doc::Cell>) -> Layer {
    Layer {
        name: String::new(),
        is_background: false,
        has_url: false,
        is_special: false,
        frame_changing: false,
        display: true,
        nesting_level: None,
        cells,
        visibility_per_frame: None,
        shared_link: None,
    }
}

fn cell(ids: Vec<ObjectId>) -> doc::Cell {
    doc::Cell {
        locked: false,
        visible: true,
        elements: ids,
    }
}

/// An image object over the canvas origin, `extent` wide and tall.
fn image(registry: &mut Registry, extent: (i64, i64), tile: u32) -> ObjectId {
    image_at(registry, extent, Point::new(0.0, 0.0), tile)
}

/// An image object placed at `location`.
fn image_at(registry: &mut Registry, extent: (i64, i64), location: Point, tile: u32) -> ObjectId {
    registry.insert(ObjectData::Image(doc::ImageObject {
        location,
        offset: Point::new(0.0, 0.0),
        pixels: extent,
        tile_size: raster::TILE_SIDE as i64,
        tiles: vec![TileRef::Tile(tile)],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }))
}

/// A tile of one flat color.
fn solid(color: [u8; 4]) -> Tile {
    let mut pixels = [0u8; TILE_BYTES];
    for px in pixels.as_chunks_mut::<4>().0.iter_mut() {
        px.copy_from_slice(&color);
    }
    Tile::from_rgba8(pixels)
}

/// A tile whose gray level steps at `split`: the columns left of it
/// carry `left`, the rest carry `right`. The alpha is opaque — a
/// grayscale mask carries its coverage in the color channels.
fn gray_step(left: u8, right: u8, split: usize) -> Tile {
    let mut pixels = [0u8; TILE_BYTES];
    for y in 0..TILE_SIDE {
        for x in 0..TILE_SIDE {
            let gray = if x < split { left } else { right };
            let index = (y * TILE_SIDE + x) * 4;
            pixels[index] = gray;
            pixels[index + 1] = gray;
            pixels[index + 2] = gray;
            pixels[index + 3] = 255;
        }
    }
    Tile::from_rgba8(pixels)
}

/// A mask tile whose gray ramps per column: column `x` carries
/// `(x * 256 / TILE_SIDE)` clamped to 255 — a distinct value per
/// source column, so any resampling or mis-sampling is visible in the
/// output.
fn gray_ramp() -> Tile {
    let mut pixels = [0u8; TILE_BYTES];
    for y in 0..TILE_SIDE {
        for x in 0..TILE_SIDE {
            let gray = ((x * 256) / TILE_SIDE).min(255) as u8;
            let index = (y * TILE_SIDE + x) * 4;
            pixels[index] = gray;
            pixels[index + 1] = gray;
            pixels[index + 2] = gray;
            pixels[index + 3] = 255;
        }
    }
    Tile::from_rgba8(pixels)
}

/// A mask tile that is opaque white in an 8×8 block at (4..12) — the
/// block sits well inside a 16×16 clip of the tile, so a σ1.5 blur
/// (span-5 boxes) feathers its border without flattening the whole
/// extent, and the transparent surround gives the falloff room to
/// land.
fn soft_center() -> Tile {
    let mut pixels = [0u8; TILE_BYTES];
    for y in 0..TILE_SIDE {
        for x in 0..TILE_SIDE {
            let index = (y * TILE_SIDE + x) * 4;
            let opaque = (4..12).contains(&x) && (4..12).contains(&y);
            let (gray, alpha) = if opaque { (255, 255) } else { (0, 0) };
            pixels[index] = gray;
            pixels[index + 1] = gray;
            pixels[index + 2] = gray;
            pixels[index + 3] = alpha;
        }
    }
    Tile::from_rgba8(pixels)
}

/// A mask-structured group (`GRTi` 1 — grayscale) over `children`.
fn mask_group(registry: &mut Registry, children: Vec<ObjectId>) -> ObjectId {
    registry.insert(ObjectData::Group(GroupObject {
        group_type: 1,
        name: None,
        children,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        mask_linked: None,
        stack_mode: None,
        effects: Vec::new(),
    }))
}

/// The same group, with the given combine mode.
fn mask_group_with_mode(
    registry: &mut Registry,
    children: Vec<ObjectId>,
    mode: MaskCombineMode,
) -> ObjectId {
    let id = mask_group(registry, children);
    if let Some(ObjectData::Group(group)) = registry.get_mut(id) {
        group.stack_mode = Some(mode);
    }
    id
}

/// A regular group (`GRTi` 0) — the shape a disabled mask takes.
fn regular_group(registry: &mut Registry, children: Vec<ObjectId>, mask_hidden: bool) -> ObjectId {
    registry.insert(ObjectData::Group(GroupObject {
        group_type: 0,
        name: None,
        children,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden,
        mask_linked: None,
        stack_mode: None,
        effects: Vec::new(),
    }))
}

fn pixel(render: &compose::Render, x: u32, y: u32) -> [u8; 4] {
    let index = (y * u32::from(render.width) + x) as usize * 4;
    render.pixels[index..index + 4].try_into().expect("pixel")
}

// --- proofs --------------------------------------------------------------

/// A grayscale mask restricts its content: the white half of the mask
/// shows the content, the black half shows the canvas through it.
#[test]
fn grayscale_mask_restricts_its_content() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(255, 0, 2));
    let content = image(&mut doc.registry, (4, 4), 0);
    let mask = image(&mut doc.registry, (4, 4), 1);
    let group = mask_group(&mut doc.registry, vec![mask, content]);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "inside the white half");
    assert_eq!(pixel(&render, 1, 3), RED, "inside the white half, lower");
    assert_eq!(pixel(&render, 2, 0), BACKGROUND, "inside the black half");
    assert_eq!(pixel(&render, 3, 3), BACKGROUND, "inside the black half");
}

/// The mask child is not composited as content: a mask restricts, it
/// does not paint. The gray of the mask never reaches the canvas.
#[test]
fn the_mask_artwork_is_not_composited() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    // An opaque white mask over a half-size content: composited as
    // artwork, the mask would white out the whole canvas.
    doc.tiles.insert(1, gray_step(255, 255, 2));
    let content = image(&mut doc.registry, (2, 2), 0);
    let mask = image(&mut doc.registry, (4, 4), 1);
    let group = mask_group(&mut doc.registry, vec![mask, content]);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "the content, unmasked");
    assert_eq!(pixel(&render, 3, 3), BACKGROUND, "no mask artwork");
}

/// The content's own alpha scales with the coverage: a half-white
/// mask halves the content's alpha.
#[test]
fn partial_coverage_scales_the_content_alpha() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(128, 128, 2));
    let content = image(&mut doc.registry, (4, 4), 0);
    let mask = image(&mut doc.registry, (4, 4), 1);
    let group = mask_group(&mut doc.registry, vec![mask, content]);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    let covered = pixel(&render, 0, 0);
    // Premultiplied red at half coverage over the white matte.
    assert_eq!(covered[0], 255, "the matte shows through the red");
    assert_eq!(covered[3], 255, "the matte is opaque");
    assert!(covered[1] > 100 && covered[1] < 160, "half-covered red");
}

/// The stack's levels fold: the bottom level replaces the coverage,
/// and every level above combines with the coverage below it.
#[test]
fn stack_levels_fold_with_their_combine_mode() {
    // The bottom level covers the left half; the top level covers the
    // right half. The content is red across the canvas.
    let build = |mode: MaskCombineMode| {
        let mut doc = Document::default();
        doc.tiles.insert(0, solid(RED));
        doc.tiles.insert(1, gray_step(255, 0, 2));
        doc.tiles.insert(2, gray_step(0, 255, 2));
        let content = image(&mut doc.registry, (4, 4), 0);
        let bottom_mask = image(&mut doc.registry, (4, 4), 1);
        let top_mask = image(&mut doc.registry, (4, 4), 2);
        let bottom = mask_group(&mut doc.registry, vec![bottom_mask, content]);
        let top = mask_group_with_mode(&mut doc.registry, vec![top_mask, bottom], mode);
        doc.pages
            .push(page(4.0, 4.0, vec![layer(vec![cell(vec![top])])]));
        doc
    };

    // Add — the union of the two halves covers the whole canvas.
    let added = render(&build(MaskCombineMode::Add), 0, 0).expect("render");
    assert_eq!(pixel(&added, 0, 0), RED, "left half");
    assert_eq!(pixel(&added, 3, 0), RED, "right half");

    // Intersect — the halves do not overlap, so nothing shows.
    let intersected = render(&build(MaskCombineMode::Intersect), 0, 0).expect("render");
    assert_eq!(pixel(&intersected, 0, 0), BACKGROUND, "left half");
    assert_eq!(pixel(&intersected, 3, 0), BACKGROUND, "right half");

    // Subtract — the top level's half is cut out of the coverage
    // below, which leaves the bottom level's half.
    let subtracted = render(&build(MaskCombineMode::Subtract), 0, 0).expect("render");
    assert_eq!(pixel(&subtracted, 0, 0), RED, "left half");
    assert_eq!(pixel(&subtracted, 3, 0), BACKGROUND, "right half");

    // Invert — the top level's inverted coverage stands in for the
    // coverage below, which is the bottom level's half.
    let inverted = render(&build(MaskCombineMode::Invert), 0, 0).expect("render");
    assert_eq!(pixel(&inverted, 0, 0), RED, "left half");
    assert_eq!(pixel(&inverted, 3, 0), BACKGROUND, "right half");
}

/// A disabled mask group draws its content unmasked, and the mask's
/// own artwork stays hidden.
#[test]
fn a_disabled_mask_draws_its_content_unmasked() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(255, 0, 2));
    let content = image(&mut doc.registry, (4, 4), 0);
    let mask = image(&mut doc.registry, (4, 4), 1);
    let group = regular_group(&mut doc.registry, vec![mask, content], true);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "left half");
    assert_eq!(
        pixel(&render, 3, 0),
        RED,
        "right half: the mask does not apply"
    );
}

/// A plain group that holds an image first is not a disabled mask: it
/// draws every child, as it always did.
#[test]
fn a_plain_group_draws_every_child() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(255, 0, 2));
    // The content covers only the left half, so the image first shows
    // through on the right.
    let content = image(&mut doc.registry, (2, 2), 0);
    let picture = image(&mut doc.registry, (4, 4), 1);
    let group = regular_group(&mut doc.registry, vec![picture, content], false);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "the content");
    assert_eq!(
        pixel(&render, 3, 0),
        [0, 0, 0, 255],
        "the image first still draws"
    );
}

/// A disabled level of a stack contributes no coverage: the level
/// below it applies on its own.
#[test]
fn a_disabled_level_in_a_stack_is_skipped() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    // The bottom level covers the left half; the top level covers the
    // right half and intersects it, so an enabled stack shows nothing.
    doc.tiles.insert(1, gray_step(255, 0, 2));
    doc.tiles.insert(2, gray_step(0, 255, 2));
    let content = image(&mut doc.registry, (4, 4), 0);
    let bottom_mask = image(&mut doc.registry, (4, 4), 1);
    let top_mask = image(&mut doc.registry, (4, 4), 2);
    let bottom = mask_group(&mut doc.registry, vec![bottom_mask, content]);
    let top = mask_group_with_mode(
        &mut doc.registry,
        vec![top_mask, bottom],
        MaskCombineMode::Intersect,
    );
    // The top level is disabled: the shape `mask.enable` leaves
    // behind.
    let disabled = regular_group(&mut doc.registry, vec![top_mask, bottom], true);
    assert_ne!(disabled, top, "the disabled level is a distinct group");
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![disabled])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "the bottom level applies");
    assert_eq!(pixel(&render, 3, 0), BACKGROUND, "the top level is skipped");
}

/// A disabled top level still lets the level below it apply.
#[test]
fn a_disabled_top_level_still_applies_the_level_below() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(255, 0, 2));
    let content = image(&mut doc.registry, (4, 4), 0);
    let bottom_mask = image(&mut doc.registry, (4, 4), 1);
    let bottom = mask_group(&mut doc.registry, vec![bottom_mask, content]);
    // A disabled level whose content is the level below.
    let top_mask = image(&mut doc.registry, (4, 4), 1);
    let disabled = regular_group(&mut doc.registry, vec![top_mask, bottom], true);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![disabled])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "the bottom level applies");
    assert_eq!(pixel(&render, 3, 0), BACKGROUND, "outside the coverage");
}

// --- native-resolution sampling (issue #114) -----------------------------

/// The coverage the stack applied at `(x, y)`: opaque red content over
/// the white matte leaves `255 − green` as the coverage. Where the
/// content does not paint this reads 0 (the matte), which is also what
/// "outside the mask's footprint" means.
fn coverage_at(render: &compose::Render, x: u32, y: u32) -> u8 {
    255 - pixel(render, x, y)[1]
}

/// A raster mask is sampled on its own `WPXi`×`HPXi` grid: the same
/// 4×4 mask over the same 4×4 content renders byte-identically on a
/// canvas the size of the mask and on a canvas twice as large, and the
/// native ramp columns come through exactly (a canvas-space resample
/// would land between them).
#[test]
fn mask_sampling_is_invariant_to_the_canvas_size() {
    let build = |canvas: f64| {
        let mut doc = Document::default();
        doc.tiles.insert(0, solid(RED));
        doc.tiles.insert(1, gray_ramp());
        let content = image(&mut doc.registry, (4, 4), 0);
        let mask = image(&mut doc.registry, (4, 4), 1);
        let group = mask_group(&mut doc.registry, vec![mask, content]);
        doc.pages
            .push(page(canvas, canvas, vec![layer(vec![cell(vec![group])])]));
        render(&doc, 0, 0).expect("render")
    };
    let small = build(4.0);
    let large = build(8.0);
    for y in 0..4u32 {
        for x in 0..4u32 {
            assert_eq!(
                pixel(&small, x, y),
                pixel(&large, x, y),
                "the mask's own-resolution region changed with the canvas at ({x},{y})"
            );
        }
    }
    // The ramp's native columns carry gray 0, 2, 4, 6 — exact.
    for y in 0..4u32 {
        assert_eq!(coverage_at(&small, 0, y), 0, "ramp column 0");
        assert_eq!(coverage_at(&small, 1, y), 2, "ramp column 1");
        assert_eq!(coverage_at(&small, 2, y), 4, "ramp column 2");
        assert_eq!(coverage_at(&small, 3, y), 6, "ramp column 3");
    }
    // Past the content's footprint the larger canvas shows the matte.
    for y in 0..8u32 {
        for x in 0..8u32 {
            if x < 4 && y < 4 {
                continue;
            }
            assert_eq!(
                pixel(&large, x, y),
                BACKGROUND,
                "content leaked past its footprint at ({x},{y})"
            );
        }
    }
}

/// The mask image's own opacity (`OPAi`) scales the coverage: a 50%
/// white mask covers about half. Before issue #114 the gray restore
/// divided the alpha back out and this read 255 — a no-op mask.
#[test]
fn a_mask_image_opacity_scales_the_coverage() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(255, 255, 4)); // full white
    let content = image(&mut doc.registry, (4, 4), 0);
    let mask = image(&mut doc.registry, (4, 4), 1);
    if let Some(ObjectData::Image(image)) = doc.registry.get_mut(mask) {
        image.opacity = 500;
    }
    let group = mask_group(&mut doc.registry, vec![mask, content]);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    let cover = coverage_at(&render, 0, 0);
    assert!(
        (126..=129).contains(&cover),
        "50% mask opacity gave coverage {cover}, expected ~127"
    );
}

/// A live effect on the mask (a blur feathering the artwork's soft
/// edge) computes on the mask's own extent — the coverage is
/// byte-identical between canvas sizes, and the edge is feathered
/// (strictly between 0 and 255), not the hard cut an unblurred mask
/// paints.
#[test]
fn a_blurred_mask_samples_at_its_own_resolution() {
    let build = |canvas: f64| {
        let mut doc = Document::default();
        doc.tiles.insert(0, solid(RED));
        doc.tiles.insert(1, soft_center());
        let content = image(&mut doc.registry, (16, 16), 0);
        let mask = image(&mut doc.registry, (16, 16), 1);
        if let Some(ObjectData::Image(image)) = doc.registry.get_mut(mask) {
            image.effects = vec![doc::Effect {
                category: String::new(),
                internal_name: "gaussianBlur".to_owned(),
                properties: vec![doc::EffectProperty {
                    key: "gaussian_blur_radius".to_owned(),
                    value: doc::EffectValue::Text("1.5".to_owned()),
                }],
            }];
        }
        let group = mask_group(&mut doc.registry, vec![mask, content]);
        doc.pages
            .push(page(canvas, canvas, vec![layer(vec![cell(vec![group])])]));
        render(&doc, 0, 0).expect("render")
    };
    let small = build(16.0);
    let large = build(32.0);
    for y in 0..16u32 {
        for x in 0..16u32 {
            assert_eq!(
                pixel(&small, x, y),
                pixel(&large, x, y),
                "the blurred mask changed with the canvas at ({x},{y})"
            );
        }
    }
    let edge = coverage_at(&small, 0, 8);
    let center = coverage_at(&small, 8, 8);
    assert!(
        (1..255).contains(&edge),
        "the blur did not feather the mask's edge: coverage {edge}"
    );
    assert!(
        center > edge,
        "the artwork's center should stay more covered than its edge: center {center}, edge {edge}"
    );
}

/// The mask's footprint lands at its rounded `location + offset` —
/// the same rounding the canvas-space assemble clipped by — and the
/// region the mask does not cover stays uncovered.
#[test]
fn the_mask_footprint_lands_at_its_rounded_location() {
    let mut doc = Document::default();
    doc.tiles.insert(0, solid(RED));
    doc.tiles.insert(1, gray_step(255, 0, 2));
    let content = image(&mut doc.registry, (4, 4), 0);
    let mask = image_at(&mut doc.registry, (4, 4), Point::new(0.5, 0.0), 1);
    let group = mask_group(&mut doc.registry, vec![mask, content]);
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    // 0.5 rounds to 1: canvas x0 lies outside the footprint, x1..x2
    // read the mask's white half, x3 its black half.
    assert_eq!(pixel(&render, 0, 0), BACKGROUND, "outside the footprint");
    assert_eq!(pixel(&render, 1, 0), RED, "the mask's white half");
    assert_eq!(pixel(&render, 2, 0), RED, "the mask's white half");
    assert_eq!(pixel(&render, 3, 0), BACKGROUND, "the mask's black half");
}

/// A mask extent beyond the native sampling ceiling (one side past the
/// rasterizer's `u16` extent, one area past the pixel cap) must never
/// abort a render that used to succeed: sampling falls back to the
/// canvas, where the extent clips to the canvas.
#[test]
fn a_mask_extent_beyond_the_sampling_ceiling_still_renders() {
    for extent in [(70_000, 4), (12_000, 6_000)] {
        let mut doc = Document::default();
        doc.tiles.insert(0, solid(RED));
        doc.tiles.insert(1, gray_step(255, 0, 2));
        let content = image(&mut doc.registry, (4, 4), 0);
        let mask = image(&mut doc.registry, extent, 1);
        let group = mask_group(&mut doc.registry, vec![mask, content]);
        doc.pages
            .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

        let render = render(&doc, 0, 0)
            .unwrap_or_else(|error| panic!("extent {extent:?} must still render: {error}"));
        assert_eq!(pixel(&render, 0, 0), RED, "extent {extent:?} left half");
        assert_eq!(
            pixel(&render, 3, 0),
            BACKGROUND,
            "extent {extent:?} right half"
        );
    }
}
