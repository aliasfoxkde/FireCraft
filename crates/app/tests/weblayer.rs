//! The M11 task 5 stopping condition: the shell's apply path drives
//! slice-create → link → export end to end, through the same
//! `app_cli::ops::export_web` pipeline the CLI verb and the export
//! dialog share — byte-equal to calling `webexport::export` directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::{Editor, Tool};

/// The web-layer slice `end_drag` minted, if any.
fn created_slice(document: &doc::Document) -> doc::ObjectId {
    document.pages[0]
        .layers
        .layers
        .iter()
        .find(|layer| layer.has_url && layer.is_special)
        .and_then(|layer| layer.cells[0].elements.first())
        .copied()
        .expect("slice.create put one slice on the web layer")
}

#[test]
fn slice_tool_drag_creates_and_links_then_exports() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Slice;
    editor.begin_drag(geom::Point::new(10.0, 10.0));
    editor.drag_to(geom::Point::new(90.0, 50.0));
    editor.end_drag().expect("slice.create applies");

    // The drag frame landed on the web layer, normalized.
    let id = created_slice(editor.document());
    let Some(doc::ObjectData::Slice(slice)) = editor.document().registry.get(id) else {
        panic!("created object is a slice");
    };
    assert_eq!(slice.frame, geom::Rect::new(10.0, 10.0, 90.0, 50.0));
    assert_eq!(slice.slice_type, Some(0));

    // Link editing through the same registry id the panel drives.
    editor
        .apply(
            "slice.setLink",
            serde_json::json!({ "id": id, "link": "https://example.test/", "alt": "go" }),
        )
        .expect("slice.setLink applies");

    // Export end to end through the shared CLI ops path.
    let options = webexport::WebExportOptions::default();
    let exported = app_cli::ops::export_web(
        editor.document(),
        &options,
        webexport::Workflow::HtmlAndImages,
    )
    .expect("web export runs");
    let html = exported.html.as_deref().unwrap();
    assert!(html.contains(r#"<a href="https://example.test/""#));
    assert!(html.contains("alt=\"go\""));
    // Byte-equal to the pure library call (no surface drift).
    let direct = webexport::export(
        editor.document(),
        &options,
        webexport::Workflow::HtmlAndImages,
    )
    .unwrap();
    assert_eq!(exported, direct);
}

#[test]
fn hotspot_tool_drag_creates_a_map_only_area() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Hotspot;
    editor.begin_drag(geom::Point::new(4.0, 4.0));
    editor.drag_to(geom::Point::new(20.0, 12.0));
    editor.end_drag().expect("hotspot.create applies");

    let id = created_slice(editor.document());
    let Some(doc::ObjectData::Slice(slice)) = editor.document().registry.get(id) else {
        panic!("created object is a hotspot");
    };
    // Hotspots carry no `TSLi` — map-only overlays at export.
    assert_eq!(slice.slice_type, None);
    let exported = app_cli::ops::export_web(
        editor.document(),
        &webexport::WebExportOptions::default(),
        webexport::Workflow::HtmlAndImages,
    )
    .unwrap();
    let html = exported.html.as_deref().unwrap();
    assert!(html.contains("<area shape=\"rect\""));
}

#[test]
fn degenerate_drags_create_nothing() {
    let mut editor = Editor::untitled();
    for tool in [Tool::Slice, Tool::Hotspot] {
        editor.tool = tool;
        editor.begin_drag(geom::Point::new(8.0, 8.0));
        editor.drag_to(geom::Point::new(8.2, 8.1));
        editor.end_drag().expect("a click is not a shape");
    }
    let document = editor.document();
    let empty = document.pages[0]
        .layers
        .layers
        .iter()
        .find(|layer| layer.has_url && layer.is_special)
        .is_none_or(|layer| layer.cells[0].elements.is_empty());
    assert!(empty, "no web objects for sub-pixel drags");
}

#[test]
fn every_web_workflow_exports_through_the_shared_ops_path() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Slice;
    editor.begin_drag(geom::Point::new(0.0, 0.0));
    editor.drag_to(geom::Point::new(32.0, 16.0));
    editor.end_drag().unwrap();
    let options = webexport::WebExportOptions::default();
    for workflow in [
        webexport::Workflow::ImagesOnly,
        webexport::Workflow::HtmlAndImages,
        webexport::Workflow::CssAndImages,
        webexport::Workflow::CssSprites,
        webexport::Workflow::CssProperties,
        webexport::Workflow::PagesToFiles,
        webexport::Workflow::StatesToFiles,
        webexport::Workflow::LayersToFiles,
        webexport::Workflow::StatesToGif,
    ] {
        let first = app_cli::ops::export_web(editor.document(), &options, workflow).unwrap();
        let second = app_cli::ops::export_web(editor.document(), &options, workflow).unwrap();
        assert_eq!(first, second, "{workflow:?} is not byte-deterministic");
    }
}
