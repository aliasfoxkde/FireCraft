//! The web export's behavior half, end to end: a behavior attached
//! through the engine's command surface reaches the exported HTML as
//! the MM_* markup, and the file it swaps to is one the export
//! actually wrote.
//!
//! The exporter test (`webexport`) pins the markup, but it builds the
//! behavior on the model. This drives the same markup from the
//! command surface the shell drives — `slice.create`,
//! `selection.set`, `slice.addSwapImageBehaviorFromPoint`,
//! `slice.addBehavior`, `slice.setData` — through the export door the
//! CLI verb and the Export dialog share (`app_cli::ops::export_web`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app_cli::ops::export_web;
use engine::CommandRegistry;
use engine::catalog;
use webexport::{WebExportOptions, Workflow};

/// The engine's command registry with the built-in catalog installed.
fn registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();
    let _ = catalog::install(&mut reg);
    reg
}

/// An 8x4 canvas of ink — red on the left, blue on the right — as
/// the raster import path builds it.
fn fixture() -> doc::Document {
    let (width, height) = (8u32, 4u32);
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let pixel = if x < 4 {
                [255u8, 0, 0, 255]
            } else {
                [0u8, 0, 0, 255]
            };
            rgba.extend_from_slice(&pixel);
        }
    }
    io::raster::document_from_image(
        &io::raster::DecodedImage {
            width,
            height,
            rgba,
            source: io::raster::SourceFormat::Png,
        },
        Some("behaviors".to_owned()),
    )
    .expect("fixture document")
}

/// The id of the slice the document carries under `name`.
fn slice_id(document: &doc::Document, name: &str) -> doc::ObjectId {
    document
        .registry
        .iter()
        .find_map(|(id, data)| match data {
            doc::ObjectData::Slice(slice) if slice.name.as_deref() == Some(name) => Some(id),
            _ => None,
        })
        .expect("the named slice")
}

/// The payload form of an object id.
fn id_json(id: doc::ObjectId) -> serde_json::Value {
    serde_json::json!({ "index": id.index(), "generation": id.generation() })
}

/// Two slices on the page's Web Layer, created through the command
/// surface (which provisions the layer itself).
fn sliced(registry: &CommandRegistry, document: &mut doc::Document) {
    for (name, x) in [("left", 0.0), ("right", 4.0)] {
        registry
            .execute(
                document,
                "slice.create",
                serde_json::json!({
                    "shape": "rect", "x": x, "y": 0.0,
                    "width": 4.0, "height": 4.0, "page": 0, "name": name,
                }),
            )
            .expect("slice.create");
    }
}

#[test]
fn a_swap_behavior_attached_through_the_command_surface_reaches_the_html() {
    let mut document = fixture();
    let registry = registry();
    sliced(&registry, &mut document);

    // CS6's "add swap image behavior from point" verb: the selected
    // slice gains the behavior, the slice under the point is the
    // swap's source.
    let left = slice_id(&document, "left");
    registry
        .execute(
            &mut document,
            "selection.set",
            serde_json::json!({ "ids": [id_json(left)] }),
        )
        .expect("selection.set");
    registry
        .execute(
            &mut document,
            "slice.addSwapImageBehaviorFromPoint",
            serde_json::json!({ "where": { "x": 6.0, "y": 2.0 } }),
        )
        .expect("addSwapImageBehaviorFromPoint");

    let exported = export_web(
        &document,
        &WebExportOptions::default(),
        Workflow::HtmlAndImages,
    )
    .expect("web export");
    let html = exported.html.as_deref().expect("the HTML document");

    // The behavior's markup rides the export.
    assert!(html.contains("function MM_swapImage()"), "{html}");
    assert!(
        html.contains("onMouseOver=\"MM_swapImage('left','','right.png',1);\""),
        "{html}"
    );
    assert!(
        html.contains("onMouseOut=\"MM_swapImgRestore();\""),
        "{html}"
    );
    // The target is the file the export wrote for the slice the
    // behavior names — not the bare name, which no file carries.
    assert!(
        html.contains("onload=\"MM_preloadImages('right.png')\""),
        "{html}"
    );
    assert!(
        exported.files.iter().any(|file| file.name == "right.png"),
        "the swap target is among the exported files: {:?}",
        exported
            .files
            .iter()
            .map(|file| file.name.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_typed_file_name_target_is_emitted_verbatim() {
    let mut document = fixture();
    let registry = registry();
    sliced(&registry, &mut document);

    // The other command path: the behavior and its target written
    // explicitly, with a file name the user typed.
    let left = slice_id(&document, "left");
    registry
        .execute(
            &mut document,
            "slice.addBehavior",
            serde_json::json!({ "id": id_json(left), "kind": 2 }),
        )
        .expect("slice.addBehavior");
    registry
        .execute(
            &mut document,
            "slice.setData",
            serde_json::json!({
                "id": id_json(left), "key": "swapTarget", "value": "hero.png",
            }),
        )
        .expect("slice.setData");

    let exported = export_web(
        &document,
        &WebExportOptions::default(),
        Workflow::HtmlAndImages,
    )
    .expect("web export");
    let html = exported.html.as_deref().expect("the HTML document");

    assert!(
        html.contains("onMouseOver=\"MM_swapImage('left','','hero.png',1);\""),
        "{html}"
    );
    assert!(
        html.contains("onload=\"MM_preloadImages('hero.png')\""),
        "{html}"
    );
}
