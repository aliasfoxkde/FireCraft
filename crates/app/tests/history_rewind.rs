//! The History panel's rewind through the editor's door: the app's
//! command door routes the session-scoped `history.rewind` id, and the
//! document returns to the state the clicked step recorded.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use app::editor::Editor;
use app_cli::mcp;
use base64::Engine as _;
use engine::Session;
use engine::platform::{FileSystem, MemoryFs};

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
    editor
        .current_cell()
        .expect("cell")
        .elements
        .last()
        .copied()
        .expect("id")
}

/// The undo depth a fresh editor already has.
///
/// [`Editor::untitled`] seeds the document through two registry commands
/// (`page.add`, `page.setCanvasSize`), so its undo stack is not empty:
/// the depths a rewind names are relative to that base, not to zero.
fn base_depth(editor: &Editor) -> usize {
    editor.session.undo_len()
}

#[test]
fn the_editor_door_routes_the_history_rewind_command() {
    let mut editor = Editor::untitled();
    let base = base_depth(&editor);
    import_raster(&mut editor, "red");
    let after_first = editor.current_cell().expect("cell").elements.clone();
    import_raster(&mut editor, "green");
    assert_eq!(editor.session.undo_len(), base + 2);

    editor
        .apply("history.rewind", serde_json::json!({ "depth": base + 1 }))
        .expect("rewind");
    assert_eq!(editor.session.undo_len(), base + 1);

    // The page holds what it held after the first raster: the rewound
    // step's effect is gone.
    assert_eq!(editor.current_cell().expect("cell").elements, after_first);
}

#[test]
fn a_rewind_to_the_opened_document_drops_the_whole_edit_history() {
    let mut editor = Editor::untitled();
    let base = base_depth(&editor);
    import_raster(&mut editor, "red");
    import_raster(&mut editor, "green");
    assert_eq!(editor.session.undo_len(), base + 2);

    // Rewinding to the base depth drops the edits and keeps the seeded
    // document — the page, its layer and its canvas — intact.
    editor
        .apply("history.rewind", serde_json::json!({ "depth": base }))
        .expect("rewind to the seeded document");
    assert_eq!(editor.session.undo_len(), base);
    assert!(editor.current_cell().expect("cell").elements.is_empty());
}

#[test]
fn a_rewind_past_the_recorded_depth_is_refused() {
    let mut editor = Editor::untitled();
    let base = base_depth(&editor);
    import_raster(&mut editor, "red");
    let error = editor
        .apply("history.rewind", serde_json::json!({ "depth": base + 9 }))
        .expect_err("past the end refused");
    assert!(
        matches!(error, engine::EngineError::InvalidState { .. }),
        "{error}"
    );
    assert_eq!(editor.session.undo_len(), base + 1);
}

/// The MCP surface names the session-scoped rewind and runs it: a
/// command an agent cannot discover is a command the agent cannot
/// use.
#[test]
fn the_mcp_surface_lists_and_runs_the_rewind() {
    let base = Editor::untitled().session.document().clone();
    let mut session = Session::new(base);

    let call = |session: &mut Session, name: &str, arguments: serde_json::Value| {
        mcp::handle(
            session,
            Rc::new(MemoryFs::new()) as Rc<dyn FileSystem>,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": name, "arguments": arguments },
            }),
        )
        .expect("tools/call answers")
    };

    // The discovery list carries the session-scoped id beside the
    // registry's.
    let reply = call(
        &mut session,
        "firecraft.listCommands",
        serde_json::json!({}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        text.split('\n').any(|line| line == engine::REWIND_ID),
        "the rewind is listed: {text}"
    );

    // And it runs: one step of the undo stack is taken back.
    let before = session.undo_len();
    session
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect", "x": 10.0, "y": 20.0,
                "width": 80.0, "height": 40.0,
                "page": 0, "layer": 0, "cell": 0, "name": "plate",
            }),
        )
        .expect("create");
    assert_eq!(session.undo_len(), before + 1);

    let reply = call(
        &mut session,
        "firecraft.apply",
        serde_json::json!({
            "command": engine::REWIND_ID,
            "args": { "depth": before },
        }),
    );
    assert_eq!(
        reply["result"]["isError"],
        serde_json::json!(false),
        "{reply}"
    );
    assert_eq!(session.undo_len(), before);
}
