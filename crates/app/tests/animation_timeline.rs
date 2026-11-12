//! Phase 20.3: the animation timeline the States panel drives.
//!
//! The panel writes through the editor, so the workflow it offers is
//! exercised here: a state strip whose delays reach the page's record
//! (and the GIF exporter behind it), the onion-skinning toggle the
//! canvas reads, and the CS6 animation steps — add, duplicate, delete,
//! distribute to states.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::Editor;
use base64::Engine as _;
use compose::Render;
use engine::error::EngineError;
use serde_json::json;

/// A 2×2 opaque plane in one colour, base64 for the importer's fast
/// path.
fn plane(r: u8, g: u8, b: u8) -> String {
    let bytes: Vec<u8> = vec![r, g, b, 255, r, g, b, 255, r, g, b, 255, r, g, b, 255];
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Places a 2×2 square at `(x, 0)` in one state of the page's active
/// layer.
fn import(editor: &mut Editor, cell: usize, x: f64, colour: &str) {
    editor
        .apply(
            "bitmap.importFile",
            json!({
                "width": 2, "height": 2, "pixels": colour,
                "bounds": { "left": x, "top": 0.0, "right": x, "bottom": 0.0 },
                "maintain_aspect_ratio": true,
                "page": 0, "layer": 0, "cell": cell, "name": "square",
            }),
        )
        .unwrap();
}

/// The colour at `(x, y)` of a render, without the alpha channel.
fn pixel(render: &Render, x: usize, y: usize) -> [u8; 3] {
    let index = (y * usize::from(render.width) + x) * 4;
    [
        render.pixels[index],
        render.pixels[index + 1],
        render.pixels[index + 2],
    ]
}

#[test]
fn a_state_delay_persists_in_the_page_record() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "frame.add",
            json!({ "page": 0, "how_many": 2, "location": "end" }),
        )
        .unwrap();
    assert_eq!(editor.document().pages[0].info.frame_count, 3);

    editor
        .apply(
            "frame.setDuration",
            json!({ "page": 0, "frame": 1, "duration": 25 }),
        )
        .unwrap();
    // The record is positional and covers every state: the untouched
    // states keep the format default.
    assert_eq!(
        editor.document().pages[0].info.frame_durations,
        vec![10, 25, 10],
        "the delay reached the page's record"
    );

    // Undo restores the page's own record.
    editor.undo().unwrap();
    assert!(
        editor.document().pages[0].info.frame_durations.is_empty(),
        "undo took the delay back"
    );
}

#[test]
fn the_states_add_duplicate_and_delete() {
    let mut editor = Editor::untitled();
    assert_eq!(editor.document().pages[0].info.frame_count, 1);

    editor
        .apply(
            "frame.add",
            json!({
                "page": 0, "how_many": 1,
                "location": "after current", "current": 0,
            }),
        )
        .unwrap();
    assert_eq!(editor.document().pages[0].info.frame_count, 2);

    editor
        .apply(
            "frame.duplicate",
            json!({
                "page": 0, "index": 0, "how_many": 1,
                "location": "after current", "current": 0,
            }),
        )
        .unwrap();
    assert_eq!(editor.document().pages[0].info.frame_count, 3);

    editor
        .apply(
            "frame.delete",
            json!({ "page": 0, "index": 1, "how_many": 1 }),
        )
        .unwrap();
    assert_eq!(editor.document().pages[0].info.frame_count, 2);

    editor
        .apply(
            "frame.delete",
            json!({ "page": 0, "index": 0, "how_many": 1 }),
        )
        .unwrap();
    assert_eq!(editor.document().pages[0].info.frame_count, 1);

    // The page keeps its last state: the engine refuses to delete it.
    let error = editor
        .apply(
            "frame.delete",
            json!({ "page": 0, "index": 0, "how_many": 1 }),
        )
        .unwrap_err();
    assert!(matches!(error, EngineError::InvalidState { .. }));
    assert_eq!(editor.document().pages[0].info.frame_count, 1);
}

#[test]
fn the_onion_toggle_ghosts_the_neighbouring_state() {
    let mut editor = Editor::untitled();
    editor
        .apply(
            "frame.add",
            json!({ "page": 0, "how_many": 2, "location": "end" }),
        )
        .unwrap();
    // State 0 carries a red square, state 1 a blue one further right.
    import(&mut editor, 0, 0.0, &plane(255, 0, 0));
    import(&mut editor, 1, 8.0, &plane(0, 0, 255));

    // Without onion skinning the canvas shows the current state alone.
    let plain = compose::render(editor.document(), 0, 1).unwrap();
    assert_eq!(
        pixel(&plain, 0, 0),
        [255, 255, 255],
        "state 0's square is not in state 1"
    );

    // The toggle is the page's own setting — the compositor and the
    // States panel read the same counts.
    editor
        .apply(
            "page.setOnionSkinning",
            json!({ "page": 0, "before": 1, "after": 0 }),
        )
        .unwrap();
    assert_eq!(editor.document().pages[0].info.onion_before, 1);
    assert_eq!(editor.document().pages[0].info.onion_after, 0);

    let onion = compose::render_onion(editor.document(), 0, 1, 1.0).unwrap();
    assert_eq!(onion.width, plain.width);
    assert_eq!(onion.height, plain.height);
    // The neighbour ghosts at half alpha over the matte: the red over
    // the white canvas.
    assert_eq!(
        pixel(&onion, 0, 0),
        [255, 128, 128],
        "the previous state ghosts behind the current one"
    );
    // The current state is opaque and untouched.
    assert_eq!(pixel(&onion, 8, 0), [0, 0, 255]);

    // Switching the onion off takes the ghosts back.
    editor
        .apply(
            "page.setOnionSkinning",
            json!({ "page": 0, "before": 0, "after": 0 }),
        )
        .unwrap();
    let plain_again = compose::render_onion(editor.document(), 0, 1, 1.0).unwrap();
    assert_eq!(pixel(&plain_again, 0, 0), [255, 255, 255]);
}

#[test]
fn distributing_a_layers_items_spreads_them_one_per_state() {
    let mut editor = Editor::untitled();
    // Two items in the active layer's first state — the shape CS6's
    // tweened run of instances takes before it is distributed.
    import(&mut editor, 0, 0.0, &plane(255, 0, 0));
    import(&mut editor, 0, 8.0, &plane(0, 0, 255));
    let layer = editor.active_layer();

    editor
        .apply(
            "frame.distributeLayer",
            json!({ "page": 0, "layer": layer, "cell": 0 }),
        )
        .unwrap();
    assert_eq!(
        editor.document().pages[0].info.frame_count,
        2,
        "the page grew a state for the second item"
    );
    let cells = &editor.document().pages[0].layers.layers[layer].cells;
    assert_eq!(cells[0].elements.len(), 1, "the first item stays");
    assert_eq!(cells[1].elements.len(), 1, "the second item moved on");
}
