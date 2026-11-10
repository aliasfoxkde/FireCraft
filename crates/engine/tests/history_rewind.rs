//! History rewind: the session-scoped `history.rewind` command, the
//! History panel's depth list, and the journal round-trip that keeps a
//! restarted session on the rewound document.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use support::{DocumentBytes, sample_document};

use engine::{EngineError, Session, REWIND_ID};
use serde_json::json;

fn session() -> (Session, Vec<doc::ObjectId>) {
    let (document, ids) = sample_document();
    (Session::new(document), ids)
}

fn set_opacity(session: &mut Session, id: doc::ObjectId, opacity: i64) {
    session
        .apply("object.setOpacity", json!({ "id": id, "opacity": opacity }))
        .expect("apply");
}

#[test]
fn rewind_undoes_the_steps_above_the_depth() {
    let (mut session, ids) = session();
    let original = DocumentBytes::of(session.document());

    set_opacity(&mut session, ids[0], 400);
    let after_first = DocumentBytes::of(session.document());
    set_opacity(&mut session, ids[1], 500);
    set_opacity(&mut session, ids[0], 600);
    assert_eq!(session.undo_len(), 3);

    let undone = session.rewind_to(1).expect("rewind");
    assert_eq!(undone, 2);
    assert_eq!(session.undo_len(), 1);
    assert_eq!(session.redo_len(), 0);
    assert_eq!(DocumentBytes::of(session.document()), after_first);

    // Depth 0 is the document as opened.
    session.rewind_to(0).expect("rewind to zero");
    assert_eq!(session.undo_len(), 0);
    assert_eq!(DocumentBytes::of(session.document()), original);
}

#[test]
fn a_rewind_clears_the_redo_stack() {
    let (mut session, ids) = session();
    set_opacity(&mut session, ids[0], 400);
    set_opacity(&mut session, ids[1], 500);
    session.undo().expect("undo");
    assert_eq!(session.redo_len(), 1);

    session.rewind_to(0).expect("rewind");
    assert_eq!(session.redo_len(), 0);
    assert!(
        session.redo().is_err(),
        "a rewound step must not be redoable"
    );
}

#[test]
fn a_rewind_to_the_current_depth_changes_nothing() {
    let (mut session, ids) = session();
    set_opacity(&mut session, ids[0], 400);
    set_opacity(&mut session, ids[1], 500);
    session.undo().expect("undo");
    assert_eq!(session.redo_len(), 1);
    let before = DocumentBytes::of(session.document());
    let journal_before = session.journal_len();

    let undone = session.rewind_to(1).expect("rewind to the current depth");
    assert_eq!(undone, 0);
    assert_eq!(DocumentBytes::of(session.document()), before);
    // Nothing was dropped, so the redo stack survives and the journal
    // records no line.
    assert_eq!(session.redo_len(), 1);
    assert_eq!(session.journal_len(), journal_before);
}

#[test]
fn a_rewind_past_the_recorded_depth_is_refused() {
    let (mut session, ids) = session();
    set_opacity(&mut session, ids[0], 400);
    let before = DocumentBytes::of(session.document());

    let error = session.rewind_to(5).expect_err("past the end refused");
    assert!(
        matches!(error, EngineError::InvalidState { .. }),
        "{error}"
    );
    assert_eq!(DocumentBytes::of(session.document()), before);
    assert_eq!(session.undo_len(), 1);
}

#[test]
fn the_history_panel_rows_carry_their_depth() {
    let (mut session, ids) = session();
    let entries = session.history_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].depth, 0);
    assert_eq!(entries[0].label, "Document opened");

    set_opacity(&mut session, ids[0], 400);
    set_opacity(&mut session, ids[1], 500);

    let entries = session.history_entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[1].depth, 1);
    assert_eq!(entries[1].label, "object.setOpacity");
    assert_eq!(entries[2].depth, 2);
    assert_eq!(entries[2].label, "object.setOpacity");
}

#[test]
fn the_rewind_command_is_session_scoped() {
    let (mut session, ids) = session();
    set_opacity(&mut session, ids[0], 400);
    set_opacity(&mut session, ids[1], 500);
    let after_first = DocumentBytes::of(session.document());

    // The registry catalog does not own it: a catalog command must
    // undo and redo against the document alone.
    assert!(!engine::catalog::ids().contains(&REWIND_ID));
    assert!(engine::catalog::session_ids().contains(&REWIND_ID));
    assert!(engine::catalog::all_ids().contains(&REWIND_ID));
    assert!(session.has_command(REWIND_ID));

    session
        .apply(REWIND_ID, json!({ "depth": 1 }))
        .expect("rewind through the command door");
    assert_eq!(session.undo_len(), 1);
    assert_eq!(DocumentBytes::of(session.document()), after_first);
}

#[test]
fn a_malformed_rewind_payload_is_refused() {
    let (mut session, _) = session();
    let error = session
        .apply(REWIND_ID, json!({ "depth": "one" }))
        .expect_err("bad payload refused");
    assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");

    let error = session
        .apply(REWIND_ID, json!({ "depth": 1, "extra": true }))
        .expect_err("unknown key refused");
    assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");
}

#[test]
fn a_rewound_session_reopens_on_the_same_document() {
    let (document, ids) = sample_document();
    let mut session = Session::new(document);
    set_opacity(&mut session, ids[0], 400);
    let after_first = DocumentBytes::of(session.document());
    set_opacity(&mut session, ids[1], 500);
    session.rewind_to(1).expect("rewind");
    let rewound = DocumentBytes::of(session.document());

    // The rewind is journaled, so a restarted session lands on the
    // same document — not on the pre-rewind one.
    let lines = session.journal_lines();
    let reopened = Session::reopen(session.document().clone(), &lines).expect("reopen");
    assert_eq!(DocumentBytes::of(reopened.document()), rewound);
    assert_eq!(rewound, after_first);
    assert_eq!(reopened.undo_len(), 1);
}
