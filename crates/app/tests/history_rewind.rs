//! The History panel's rewind through the editor's door: the app's
//! command door routes the session-scoped `history.rewind` id, and the
//! document returns to the state the clicked step recorded.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::Editor;
use base64::Engine as _;

/// Imports one 2×2 opaque-red raster and returns its id.
fn import_raster(editor: &mut Editor, name: &str) -> doc::ObjectId {
    let red = [
        255u8, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    let plane = base64::engine::general_purpose::STANDARD.encode(red);
    editor
        .apply(
            "bitmap.importFile",
            serde_json::json!({
                "width": 2, "height": 2, "pixels": plane,
                "bounds": { "left": 0.0, "top": 0.0, "right": 0.0, "bottom": 0.0 },
                "maintain_aspect_ratio": true,
                "page": 0, "layer": 0, "cell": 0, "name": name,
            }),
        )
        .expect("import");
    editor.current_cell().expect("cell").elements.last().copied().expect("id")
}

#[test]
fn the_editor_door_routes_the_history_rewind_command() {
    let mut editor = Editor::untitled();
    import_raster(&mut editor, "red");
    let after_first = editor.current_cell().expect("cell").elements.clone();
    import_raster(&mut editor, "green");
    assert_eq!(editor.session.undo_len(), 2);

    editor
        .apply("history.rewind", serde_json::json!({ "depth": 1 }))
        .expect("rewind");
    assert_eq!(editor.session.undo_len(), 1);

    // The page holds what it held after the first raster: the rewound
    // step's effect is gone.
    assert_eq!(editor.current_cell().expect("cell").elements, after_first);
}

#[test]
fn a_rewind_to_the_opened_document_drops_the_whole_edit_history() {
    let mut editor = Editor::untitled();
    import_raster(&mut editor, "red");
    import_raster(&mut editor, "green");
    assert_eq!(editor.session.undo_len(), 2);

    editor
        .apply("history.rewind", serde_json::json!({ "depth": 0 }))
        .expect("rewind to the opened document");
    assert_eq!(editor.session.undo_len(), 0);
    assert!(editor.current_cell().expect("cell").elements.is_empty());
}

#[test]
fn a_rewind_past_the_recorded_depth_is_refused() {
    let mut editor = Editor::untitled();
    import_raster(&mut editor, "red");
    let error = editor
        .apply("history.rewind", serde_json::json!({ "depth": 9 }))
        .expect_err("past the end refused");
    assert!(
        matches!(error, engine::EngineError::InvalidState { .. }),
        "{error}"
    );
    assert_eq!(editor.session.undo_len(), 1);
}
