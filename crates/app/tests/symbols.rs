//! The M10 task 4 stopping condition: a scripted golden session —
//! symbol create, instance place, definition edit, break apart —
//! driven through the shell's own apply path replays to a byte-equal
//! document, and the saved `.fw.png` stream re-opens to the same
//! document byte for byte.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::{Editor, NineSliceEdge, Tool};
use engine::Session;

/// The scripted golden, in order: CREATE (draw a rect, make it a
/// symbol — an instance replaces it on the page), PLACE (a second
/// instance via the Library panel's drag-to-place), EDIT-DEFINITION
/// (enter the symbol, draw inside the scope — the fresh rect lands in
/// the definition, not on the page), a nine-slice guide drag, a type
/// flip to button, then BREAK-APART of the first instance. Every
/// mutation is an `Editor::apply` — the one door the shell drives.
fn symbol_session() -> Editor {
    let mut editor = Editor::untitled();

    editor.tool = Tool::Rect;
    editor.begin_drag(geom::Point::new(10.0, 10.0));
    editor.drag_to(geom::Point::new(90.0, 50.0));
    editor.end_drag().unwrap();
    let content = editor.current_cell().unwrap().elements[0];
    editor
        .apply(
            "symbol.create",
            serde_json::json!({
                "ids": [content], "name": "badge", "symbol_type": "graphic",
            }),
        )
        .unwrap();

    editor
        .place_symbol(0, geom::Point::new(200.0, 220.0))
        .unwrap();

    editor.enter_symbol(0).unwrap();
    editor.begin_drag(geom::Point::new(100.0, 100.0));
    editor.drag_to(geom::Point::new(140.0, 130.0));
    editor.end_drag().unwrap();
    editor.exit_symbol();

    editor.enable_nine_slice(0).unwrap();
    editor
        .begin_guide_drag(0, NineSliceEdge::Top, geom::Point::new(50.0, 30.0))
        .unwrap();
    editor.drag_guide_to(geom::Point::new(52.0, 30.0));
    editor.end_guide_drag().unwrap();

    editor
        .apply(
            "symbol.setType",
            serde_json::json!({ "symbol": 0, "symbol_type": "button" }),
        )
        .unwrap();

    let instance = editor.current_cell().unwrap().elements[0];
    editor
        .apply("symbol.breakApart", serde_json::json!({ "id": instance }))
        .unwrap();
    editor
}

#[test]
fn the_golden_legs_land_where_they_claim() {
    let editor = symbol_session();

    // Break-apart released the definition's two objects as copies
    // beside the placed instance.
    let cell = editor.current_cell().unwrap();
    assert_eq!(cell.elements.len(), 3, "two copies + the placed instance");
    // The definition still owns its content, guides and all.
    let definition = &editor.document().symbols[0];
    assert_eq!(definition.layers.layers[0].cells[0].elements.len(), 2);
    let definition = &editor.document().symbols[0];
    assert_eq!(definition.symbol_type, doc::SymbolType::Button);
    assert_eq!(
        definition.nine_slice.as_ref().unwrap().controls,
        [20.0, 0.0, 0.0, 0.0],
        "the top guide moved to the dragged inset"
    );
}

#[test]
fn the_symbol_golden_replays_and_round_trips() {
    let editor = symbol_session();
    let steps = editor.steps_text();

    // Replay on a fresh base — the same `Session::reopen` path the
    // CLI's apply verb drives.
    let replayed = Session::reopen(doc::Document::default(), &steps).expect("replays");
    assert_eq!(
        serde_json::to_vec(replayed.document()).unwrap(),
        serde_json::to_vec(editor.session.document()).unwrap(),
        "replayed document must equal the original, byte for byte"
    );

    // The file boundary: save → open → save is byte-stable. The
    // reader canonically rebuilds the registry in wire order (the
    // live session's slot layout is creation order), so the
    // round-trip contract is the stream, not the model's slot table.
    let bytes = editor.save().unwrap();
    let reopened = Editor::open(&bytes).unwrap();
    assert_eq!(
        reopened.save().unwrap(),
        bytes,
        "the saved file re-emits identically"
    );
    assert_eq!(reopened.document().symbols.len(), 1);
    assert_eq!(
        reopened.document().symbols[0].layers.layers[0].cells[0]
            .elements
            .len(),
        2,
        "the definition keeps its content across the file boundary"
    );
    assert_eq!(
        reopened.document().pages[0].layers.layers[0].cells[0]
            .elements
            .len(),
        3,
        "the page keeps the broken-apart copies and the placed instance"
    );
    assert_eq!(
        reopened.document().registry.len(),
        5,
        "no orphaned objects ride along: every slot on re-open is attached"
    );

    // And the replayed session writes the same stream.
    let mut fresh = Editor::from_document(doc::Document::default());
    fresh.replay_steps(&steps).expect("replays");
    assert_eq!(fresh.save().unwrap(), bytes);
    assert_eq!(
        fresh.session.undo_len(),
        editor.session.undo_len(),
        "the replayed history is fully undoable"
    );
}
