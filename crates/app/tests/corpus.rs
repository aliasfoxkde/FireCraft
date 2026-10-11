//! The M5 shell stopping condition, as a permanent gate: open a
//! tracked corpus fixture through the editor, apply commands from the
//! tool path, save; the saved file must round-trip through open again,
//! and its journal must replay to the same document the apply loop
//! produced (surface equivalence).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::{Editor, Tool};
use doc::ObjectData;
use engine::Session;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../corpus/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn open_fixture_edit_save_round_trips() {
    let mut editor = Editor::open(&fixture("composed.fw.png")).unwrap();
    let pages_before = editor.document().pages.len();

    // A UI-shaped edit: drag the rect tool, then set opacity on the
    // new object — exactly what the shell drives from pointer events.
    editor.tool = Tool::Rect;
    editor.begin_drag(geom::Point::new(10.0, 10.0));
    editor.drag_to(geom::Point::new(80.0, 50.0));
    editor.end_drag().unwrap();
    let id = editor.current_cell().unwrap().elements[0];
    editor
        .apply(
            "object.setOpacity",
            serde_json::json!({ "id": id, "opacity": 555 }),
        )
        .unwrap();

    let bytes = editor.save().unwrap();

    // Round-trip: the saved file reopens, keeps the page count, and
    // carries the edit.
    let reopened = Editor::open(&bytes).unwrap();
    assert_eq!(reopened.document().pages.len(), pages_before);
    let cell = reopened.current_cell().unwrap().clone();
    assert!(cell.elements.contains(&id), "created object survives save");
    match reopened.document().registry.get(id) {
        Some(ObjectData::Path(path)) => assert_eq!(path.opacity, 555),
        other => panic!("wrong object kind after round-trip: {other:?}"),
    }
}

#[test]
fn opened_fixture_journal_replays_from_the_seed() {
    // Take the fixture bytes as the seed document, drive edits, then
    // reopen a *fresh copy* of the seed and replay the journal over
    // it — the two paths must agree byte-for-byte.
    let bytes = fixture("composed.fw.png");
    let mut editor = Editor::open(&bytes).unwrap();
    editor.tool = Tool::Ellipse;
    editor.begin_drag(geom::Point::new(20.0, 20.0));
    editor.drag_to(geom::Point::new(60.0, 60.0));
    editor.end_drag().unwrap();
    let journal = editor.session.journal_lines();
    assert!(!journal.is_empty());

    // A fresh document from pristine fixture bytes, advanced only by
    // replay through the engine's restart path.
    let seed = app_cli::io::open(&fixture("composed.fw.png")).unwrap();
    let replay = Session::reopen(seed.document, &journal).unwrap();

    // Same document byte-for-byte as the interactive apply loop. The
    // canonical tree is the shared serialization both surfaces write.
    let (direct_tree, _) = doc::fwmap::write(
        editor.session.document(),
        &doc::fwmap::UnknownBag::default(),
    )
    .unwrap();
    let (replay_tree, _) =
        doc::fwmap::write(replay.document(), &doc::fwmap::UnknownBag::default()).unwrap();
    assert_eq!(
        fwpng::nodetree::serialize(&direct_tree),
        fwpng::nodetree::serialize(&replay_tree)
    );
}
