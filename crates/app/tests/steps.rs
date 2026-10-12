//! The M9 task 6 stopping condition, end to end: a GUI session's
//! journal saved as a `.fwsteps` command file replays on a fresh base
//! document to a byte-equal document — and, through the full app
//! surface, to a byte-equal `.fw.png` stream with the undo history
//! rebuilt.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::{Editor, Tool};
use engine::Session;
use engine::platform::{FileSystem, MemoryFs};

/// A GUI-shaped session: the untitled seeds, a drag-created rectangle,
/// a display edit, and a rename — every row through the registry.
fn gui_session() -> Editor {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Rect;
    editor.begin_drag(geom::Point::new(10.0, 10.0));
    editor.drag_to(geom::Point::new(90.0, 50.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];
    editor
        .apply(
            "object.setOpacity",
            serde_json::json!({ "id": id, "opacity": 640 }),
        )
        .unwrap();
    editor
        .apply(
            "object.rename",
            serde_json::json!({ "id": id, "name": "plate" }),
        )
        .unwrap();
    editor
}

#[test]
fn saved_steps_replay_to_a_byte_equal_document() {
    let editor = gui_session();
    let steps = editor.steps_text();

    // The file boundary: the command file is the journal verbatim.
    let fs = MemoryFs::new();
    fs.write_file("session.fwsteps", steps.as_bytes()).unwrap();
    let lines = String::from_utf8(fs.read_file("session.fwsteps").unwrap()).unwrap();
    assert_eq!(lines, steps, "the steps file carries the journal verbatim");

    // Replay on a fresh base document — the same `Session::reopen`
    // path the CLI's apply verb drives.
    let replayed = Session::reopen(doc::Document::default(), &lines).expect("replays");
    assert_eq!(
        serde_json::to_vec(replayed.document()).unwrap(),
        serde_json::to_vec(editor.session.document()).unwrap(),
        "replayed document must equal the original, byte for byte"
    );

    // The full-app shape: the same replay inside an Editor produces a
    // byte-identical `.fw.png` stream, undo stack included.
    let mut fresh = Editor::from_document(doc::Document::default());
    fresh.replay_steps(&lines).expect("replays");
    assert_eq!(fresh.save().unwrap(), editor.save().unwrap());
    assert_eq!(
        fresh.session.undo_len(),
        editor.session.undo_len(),
        "the replayed history is fully undoable"
    );
    // The rebuilt journal carries the replayed prefix, so further
    // edits append instead of double-applying.
    assert_eq!(fresh.steps_text(), steps);
}
