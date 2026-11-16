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
    registry.insert(ObjectData::Image(doc::ImageObject {
        location: Point::new(0.0, 0.0),
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
