//! Bitmap pixel edits meet the journal (M7 task 5 stopping condition):
//! a masked-edit session replays byte-exact from a fresh tile-loaded
//! base, and undo/redo across the restart boundary lands on the same
//! bytes as the live session — selection state and tile payloads both.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use support::{Scratch, sample_bitmap_document};

use serde_json::json;

/// A full masked-edit session — marquee, feather, fill, reselect, erase,
/// wand, move, deselect — replayed from a fresh base reproduces the live
/// document bytes, and undoing everything reaches the untouched base.
#[test]
fn bitmap_edits_replay_and_undo_byte_exact() {
    let (document, image) = sample_bitmap_document();
    let mut session = Scratch::open(document);
    let original = session.bytes();

    session.apply(
        "bitmap.selectRect",
        json!({ "id": image, "x": 1, "y": 1, "width": 2, "height": 2 }),
        None,
    );
    session.apply("bitmap.feather", json!({ "radius": 1 }), None);
    session.apply(
        "bitmap.fill",
        json!({ "id": image, "color_raw": 0xFF_FF_FF_FFi64 }),
        None,
    );
    session.apply(
        "bitmap.selectEllipse",
        json!({ "id": image, "x": 0, "y": 0, "width": 4, "height": 4 }),
        None,
    );
    session.apply("bitmap.erase", json!({ "id": image }), None);
    session.apply(
        "bitmap.selectWand",
        json!({ "id": image, "x": 0, "y": 0, "tolerance": 0, "contiguous": false }),
        None,
    );
    session.apply(
        "bitmap.movePixels",
        json!({ "id": image, "dx": 1, "dy": 0 }),
        None,
    );
    session.apply("bitmap.selectNone", json!({}), None);
    let edited = session.bytes();

    // --- restart: fresh document + tiles + history, replay ---
    let mut restarted = session.restart();
    assert_eq!(restarted.bytes(), edited, "replay reproduces the pixels");
    assert_eq!(restarted.history.undo_len(), 8);
    assert_eq!(restarted.history.redo_len(), 0);

    // Undo everything across the restart boundary: the tile payloads
    // and the selection must both return to the base state.
    for _ in 0..8 {
        restarted.undo();
    }
    assert_eq!(restarted.bytes(), original, "undo reaches the base bytes");
    assert!(
        restarted.document.selection.is_none(),
        "base has no selection"
    );
    assert_eq!(restarted.history.redo_len(), 8);

    // Redo everything: byte-identical to the live session's result.
    for _ in 0..8 {
        restarted.redo();
    }
    assert_eq!(restarted.bytes(), edited, "redo closes the loop");
}

/// The live undo path and the restarted undo path are the same bytes
/// after the same prefix of undo steps — a bitmap delta must not
/// reconstruct differently depending on where it came from.
#[test]
fn bitmap_undo_paths_agree_across_restart() {
    let (document, image) = sample_bitmap_document();
    let mut live = Scratch::open(document);

    live.apply(
        "bitmap.selectRect",
        json!({ "id": image, "x": 0, "y": 0, "width": 2, "height": 4 }),
        None,
    );
    live.apply(
        "bitmap.fill",
        json!({ "id": image, "color_raw": 0xFF_00_FF_00i64 }),
        None,
    );
    live.apply(
        "bitmap.selectWand",
        json!({ "id": image, "x": 3, "y": 0, "tolerance": 30, "mode": "add" }),
        None,
    );
    live.apply("bitmap.erase", json!({ "id": image }), None);
    let edited = live.bytes();

    live.undo();
    live.undo();
    let undone = live.bytes();
    live.redo();
    live.redo();
    assert_eq!(live.bytes(), edited, "live redo closes the loop");

    let mut restarted = live.restart();
    restarted.undo();
    restarted.undo();
    assert_eq!(restarted.bytes(), undone, "restart undo == live undo");
}

/// A refused bitmap command (no loaded tile payload, non-128 grid)
/// journals nothing and leaves the session replay-identical to the
/// untouched base.
#[test]
fn refused_bitmap_edit_journals_nothing() {
    let (mut document, image) = sample_bitmap_document();
    // Detach the tile payload: the image references a tile the bank
    // no longer holds.
    document.tiles.remove(0);
    let mut session = Scratch::open(document);
    let base = session.bytes();

    let error = session
        .registry
        .execute(
            &mut session.document,
            "bitmap.fill",
            json!({ "id": image, "color_raw": 0xFF_00_00_00i64 }),
        )
        .expect_err("missing payload refuses");
    assert!(
        matches!(error, engine::EngineError::InvalidState { .. }),
        "{error}"
    );
    assert_eq!(session.journal.len(), 0, "refusals do not journal");
    assert_eq!(session.bytes(), base, "refusal leaves the bytes alone");

    let restarted = session.restart();
    assert_eq!(restarted.bytes(), base, "empty journal replays the base");
}
